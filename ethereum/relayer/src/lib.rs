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
use alloy::hex::decode;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::{Address, U256};
use alloy::providers::fillers::{
    ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::{Identity, ProviderBuilder, RootProvider, WalletProvider};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use alloy::transports::http::{Client, Http};
use async_trait::async_trait;
use bridge_core::key_store::KeyStore;
use bridge_core::relay::Relayer;
use log::error;

pub mod key_store;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Bridge,
    "../../ethereum/bridge-contracts/out/Bridge.sol/Bridge.json"
);

/// Relays bridge request to smart contracts deployed on ethereum based network.
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

impl EthereumRelayer {
    pub fn new(
        rpc_url: &str,
        bridge_address: &str,
        key_store: EthereumKeyStore,
    ) -> Result<Self, ()> {
        let signer = PrivateKeySigner::from(
            key_store
                .read()
                .map_err(|e| error!("Can't read key store: {:?}", e))?,
        );
        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(
                rpc_url
                    .parse()
                    .map_err(|e| error!("Could not parse rpc url"))?,
            );

        let bridge_instance = Bridge::new(
            Address::from_slice(
                &decode(bridge_address).map_err(|e| error!("Can't decode bridge address"))?,
            ),
            provider,
        );

        Ok(Self { bridge_instance })
    }

    pub fn get_address(&self) -> Address {
        self.bridge_instance
            .provider()
            .signer_addresses()
            .next()
            .unwrap()
    }
}

#[async_trait]
impl Relayer for EthereumRelayer {
    async fn relay(&self, data: Vec<u8>) -> Result<(), ()> {
        //todo: error handling
        //todo: amount and account from data
        let withdraw_builder = self.bridge_instance.payOut(
            U256::from_str_radix("10", 10).map_err(|e| {
                error!("Could not parse amount: {:?}", e);
            })?,
            Address::from_slice(
                &decode("0x70997970C51812dc3A010C7d01b50e0d17dc79C8").map_err(|e| {
                    error!("Could not create address: {:?}", e);
                })?,
            ),
        );
        // todo: what if relayer sends it but fails to watch ( connection lost ) ? when do we assume it's relayed ?
        // todo: fees?
        withdraw_builder
            .send()
            .await
            .map_err(|e| {
                error!("Error while sending tx: {:?}", e);
            })?
            .watch()
            .await
            .map_err(|e| {
                error!("Error while watching tx: {:?}", e);
            })?;
        Ok(())
    }
}
