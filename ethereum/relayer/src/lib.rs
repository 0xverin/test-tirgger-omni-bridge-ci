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
use alloy::providers::{Identity, PendingTransactionError, Provider, ProviderBuilder, RootProvider, WalletProvider};
use alloy::signers::k256::ecdsa::SigningKey;
use alloy::signers::local::{LocalSigner, PrivateKeySigner};
use alloy::sol;
use alloy::transports::http::{Client, Http};
use async_trait::async_trait;
use bridge_core::config::BridgeConfig;
use bridge_core::key_store::KeyStore;
use bridge_core::relay::{RelayError, Relayer};
use log::{debug, error};
use metrics::{describe_gauge, gauge};
#[cfg(test)]
use mockall::automock;
use serde::Deserialize;
use std::collections::HashMap;

pub mod key_store;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Bridge,
    "../../ethereum/chainbridge-contracts/out/Bridge.sol/Bridge.json"
);

#[async_trait]
#[cfg_attr(test, automock)]
pub trait BridgeInterface {
    async fn vote_proposal(
        &self,
        domain_id: u8,
        deposit_nonce: u64,
        resource_id: FixedBytes<32>,
        call_data: Bytes,
    ) -> Result<(), RelayError>;
}

#[async_trait]
#[cfg_attr(test, automock)]
pub trait RelayerBalance {
    async fn get_balance(&self) -> Result<u128, ()>;
}

type BridgeInstanceType = BridgeInstance<
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
>;

#[allow(clippy::type_complexity)]
pub struct BridgeContractWrapper {
    instance: BridgeInstanceType,
}

#[async_trait]
impl BridgeInterface for BridgeContractWrapper {
    async fn vote_proposal(
        &self,
        domain_id: u8,
        deposit_nonce: u64,
        resource_id: FixedBytes<32>,
        call_data: Bytes,
    ) -> Result<(), RelayError> {
        let proposal_builder = self.instance.voteProposal(domain_id, deposit_nonce, resource_id, call_data);
        let tx_hash = proposal_builder
            .send()
            .await
            .map_err(|e| {
                error!("Could not send proposal vote: {:?}", e);
                if matches!(e, alloy::contract::Error::TransportError(_)) {
                    RelayError::TransportError
                } else {
                    RelayError::Other
                }
            })?
            .watch()
            .await
            .map_err(|e| {
                error!("Could not watch proposal vote: {:?}", e);
                if matches!(e, PendingTransactionError::TransportError(_)) {
                    RelayError::TransportError
                } else {
                    RelayError::Other
                }
            })?;
        log::debug!("Submitted vote proposal, tx_hash: {:?}", tx_hash);
        Ok(())
    }
}

#[async_trait]
impl RelayerBalance for BridgeContractWrapper {
    async fn get_balance(&self) -> Result<u128, ()> {
        let address = self.instance.provider().default_signer_address();
        self.instance
            .provider()
            .get_balance(address)
            .await
            .map_err(|e| {
                log::error!("Could not get relayer balance: {}", e);
            })
            .map(|balance| balance.to())
    }
}

#[derive(Deserialize)]
pub struct RelayerConfig {
    pub node_rpc_url: String,
    pub bridge_contract_address: String,
}

pub async fn create_from_config(keystore_dir: String, config: &BridgeConfig) -> HashMap<String, Box<dyn Relayer>> {
    let mut relayers: HashMap<String, Box<dyn Relayer>> = HashMap::new();
    for relayer_config in config.relayers.iter().filter(|r| r.relayer_type == "ethereum") {
        let key_store = EthereumKeyStore::new(format!("{}/{}.bin", keystore_dir, relayer_config.id));

        let substrate_relayer_config: RelayerConfig = relayer_config.to_specific_config();

        let signer =
            PrivateKeySigner::from(key_store.read().map_err(|e| error!("Can't read key store: {:?}", e)).unwrap());
        let relayer_address = signer.address();
        log::info!("Ethereum relayer address: {:?}", relayer_address);

        let bridge_instance = prepare_bridge_instance(
            signer,
            &substrate_relayer_config.node_rpc_url,
            &substrate_relayer_config.bridge_contract_address,
        );

        let bridge_contract_wrapper = BridgeContractWrapper { instance: bridge_instance };

        let relayer: EthereumRelayer<BridgeContractWrapper> =
            EthereumRelayer::new(relayer_address.to_string(), bridge_contract_wrapper)
                .await
                .unwrap();
        relayers.insert(relayer_config.id.to_string(), Box::new(relayer));
    }
    relayers
}

/// Relays bridge request to smart contracts deployed on ethereum based network.
#[allow(clippy::type_complexity)]
pub struct EthereumRelayer<T: BridgeInterface + RelayerBalance> {
    address: String,
    bridge_instance: T,
}

// TODO: We need to configure gas options
#[allow(clippy::result_unit_err)]
impl<T: BridgeInterface + RelayerBalance> EthereumRelayer<T> {
    pub async fn new(address: String, bridge_instance: T) -> Result<Self, ()> {
        describe_gauge!(balance_gauge_name(&address), "Ethereum relayer balance");

        // initalize relayer's balance metric
        if let Ok(balance) = bridge_instance.get_balance().await {
            error!("Got balance {}", balance);
            gauge!(balance_gauge_name(&address)).set(balance as f64);
        }
        Ok(Self { address, bridge_instance })
    }
}

#[async_trait]
impl<T: BridgeInterface + RelayerBalance + Send + Sync> Relayer for EthereumRelayer<T> {
    async fn relay(&self, amount: u128, nonce: u64, resource_id: [u8; 32], data: Vec<u8>) -> Result<(), RelayError> {
        debug!("Relaying amount: {} with nonce: {} to: {:?}", amount, nonce, Address::from_slice(&data));

        // resource id 0
        let resource_id = FixedBytes::new(resource_id);

        let amount = DynSolValue::Uint(U256::from(amount), 32).abi_encode();
        let address_len = DynSolValue::Uint(U256::from(data.len()), 32).abi_encode();

        if data.len() != 20 {
            error!("Could not relay due to wrong data length");
            return Err(RelayError::Other);
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
        self.bridge_instance.vote_proposal(0, nonce, resource_id, call_data).await?;
        if let Ok(balance) = self.bridge_instance.get_balance().await {
            gauge!(balance_gauge_name(&self.address)).set(balance as f64);
        }

        debug!("Proposal relayed");
        Ok(())
    }
}

pub fn prepare_bridge_instance(
    signer: LocalSigner<SigningKey>,
    rpc_url: &str,
    bridge_contract_address: &str,
) -> BridgeInstanceType {
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url.parse().map_err(|_| error!("Could not parse rpc url")).unwrap());

    Bridge::new(
        Address::from_slice(
            &decode(bridge_contract_address)
                .map_err(|_| error!("Can't decode bridge address"))
                .unwrap(),
        ),
        provider,
    )
}

fn balance_gauge_name(address: &str) -> String {
    format!("{}_eth_balance", address)
}

#[cfg(test)]
pub mod tests {
    use crate::{
        prepare_bridge_instance, BridgeContractWrapper, BridgeInterface, EthereumRelayer, MockBridgeInterface,
        RelayerBalance,
    };
    use alloy::primitives::{Bytes, FixedBytes};
    use alloy::signers::local::PrivateKeySigner;
    use async_trait::async_trait;
    use bridge_core::relay::{RelayError, Relayer};
    use mockall::mock;

    mock! {
        BridgeInstance {}

        #[async_trait]
        impl BridgeInterface for BridgeInstance {
            async fn vote_proposal(
                &self,
                domain_id: u8,
                deposit_nonce: u64,
                resource_id: FixedBytes<32>,
                call_data: Bytes,
            ) -> Result<(), RelayError>;
        }
        #[async_trait]
        impl RelayerBalance for BridgeInstance {
            async fn get_balance(&self) -> Result<u128, ()>;
        }

    }

    #[tokio::test]
    pub async fn should_return_error_if_wrong_address_len() {
        let mut bridge_instance = MockBridgeInstance::new();
        bridge_instance.expect_vote_proposal().returning(|_, _, _, _| Ok(()));

        let relayer = EthereumRelayer::new("0x".to_string(), bridge_instance).await.unwrap();

        let result = relayer.relay(100, 1, [0; 32], [0; 32].to_vec()).await;
        assert!(matches!(result, Err(RelayError::Other)));
    }

    #[tokio::test]
    pub async fn vote_proposal_should_return_transport_error_if_node_unreachable() {
        let bridge_instance = prepare_bridge_instance(
            PrivateKeySigner::random(),
            "http://localhost:8545",
            "0x5FbDB2315678afecb367f032d93F642f64180aa3",
        );
        let wrapper = BridgeContractWrapper { instance: bridge_instance };
        let result = wrapper
            .vote_proposal(0, 1, FixedBytes::from_slice(&[0u8; 32]), Bytes::from(vec![]))
            .await;
        assert!(matches!(result, Err(RelayError::TransportError)));
    }
}
