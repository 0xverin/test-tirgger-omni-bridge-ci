// Copyright 2020-2024 Trust Computing GmbH.
// This file is part of Litentry.
//
// Litentry is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Litentry is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Litentry.  If not, see <https://www.gnu.org/licenses/>.

use crate::key_store::EthereumKeyStore;
use crate::Bridge::BridgeInstance;
use alloy::dyn_abi::DynSolValue;
use alloy::hex::decode;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::{Address, Bytes, FixedBytes, U256};
use alloy::providers::fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller};
use alloy::providers::{Identity, ProviderBuilder, RootProvider, WalletProvider};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use alloy::transports::http::{Client, Http};
use async_trait::async_trait;
use bridge_core::config::BridgeConfig;
use bridge_core::key_store::KeyStore;
use bridge_core::relay::Relayer;
use log::{debug, error};
use serde::Deserialize;
use std::collections::HashMap;

pub mod key_store;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Bridge,
    "../../ethereum/chainbridge-contracts/out/Bridge.sol/Bridge.json"
);

#[derive(Deserialize)]
pub struct RelayerConfig {
    pub node_rpc_url: String,
    pub bridge_contract_address: String,
}

pub fn create_from_config(keystore_dir: String, config: &BridgeConfig) -> HashMap<String, Box<dyn Relayer>> {
    let mut relayers: HashMap<String, Box<dyn Relayer>> = HashMap::new();
    for relayer_config in config.relayers.iter().filter(|r| r.relayer_type == "ethereum") {
        let key_store = EthereumKeyStore::new(format!("{}/{}.bin", keystore_dir, relayer_config.id));
        let substrate_relayer_config: RelayerConfig = relayer_config.to_specific_config();
        let relayer: EthereumRelayer = EthereumRelayer::new(
            &substrate_relayer_config.node_rpc_url,
            &substrate_relayer_config.bridge_contract_address,
            key_store,
        )
        .unwrap();
        relayers.insert(relayer_config.id.to_string(), Box::new(relayer));
    }
    relayers
}

/// Relays bridge request to smart contracts deployed on ethereum based network.
#[allow(clippy::type_complexity)]
pub struct EthereumRelayer {
    bridge_instance: BridgeInstance<
        Http<Client>,
        FillProvider<
            JoinFill<
                JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>,
                WalletFiller<EthereumWallet>,
            >,
            RootProvider<Http<Client>>,
            Http<Client>,
            Ethereum,
        >,
    >,
}

// TODO: We need to configure gas options
#[allow(clippy::result_unit_err)]
impl EthereumRelayer {
    pub fn new(rpc_url: &str, bridge_address: &str, key_store: EthereumKeyStore) -> Result<Self, ()> {
        let signer = PrivateKeySigner::from(key_store.read().map_err(|e| error!("Can't read key store: {:?}", e))?);

        log::info!("Ethereum relayer address: {:?}", signer.address());

        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(rpc_url.parse().map_err(|_| error!("Could not parse rpc url"))?);

        let bridge_instance = Bridge::new(
            Address::from_slice(&decode(bridge_address).map_err(|_| error!("Can't decode bridge address"))?),
            provider,
        );

        Ok(Self { bridge_instance })
    }

    pub fn get_address(&self) -> Address {
        self.bridge_instance.provider().signer_addresses().next().unwrap()
    }
}

#[async_trait]
impl Relayer for EthereumRelayer {
    async fn relay(&self, amount: u128, nonce: u64, resource_id: [u8; 32], data: Vec<u8>) -> Result<(), ()> {
        debug!("Relaying amount: {} with nonce: {} to: {:?}", amount, nonce, Address::from_slice(&data));

        // resource id 0
        let resource_id = FixedBytes::new(resource_id);

        let amount = DynSolValue::Uint(U256::from(amount), 32).abi_encode();
        let address_len = DynSolValue::Uint(U256::from(data.len()), 32).abi_encode();

        if data.len() != 20 {
            error!("Could not relay due to wrong data length");
            return Err(());
        }

        let mut address_bytes = [0; 32];
        address_bytes[0..20].copy_from_slice(&data);

        let address = DynSolValue::FixedBytes(FixedBytes(address_bytes), 32).abi_encode();

        debug!("Address bytes: {:?}", address);

        let mut bytes = vec![];

        bytes.extend(amount);
        bytes.extend(address_len);
        bytes.extend(address);

        let call_data = Bytes::copy_from_slice(&bytes);

        debug!("Call data: {:?}", call_data);

        // domainId 0 - heima
        let proposal_builder = self.bridge_instance.voteProposal(0, nonce, resource_id, call_data);

        proposal_builder.send().await.unwrap().watch().await.unwrap();
        debug!("Proposal relayed");
        Ok(())
    }
}
