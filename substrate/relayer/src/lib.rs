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

use crate::key_store::SubstrateKeyStore;
use async_trait::async_trait;
use bridge_core::key_store::KeyStore;
use bridge_core::relay::Relayer;
use bridge_core::listener::DepositRecord;
use log::error;
use std::marker::PhantomData;
use subxt::utils::AccountId32;
use subxt::{Config, OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::dev;

pub mod key_store;

// Generate an interface that we can use from the node's metadata.
#[subxt::subxt(runtime_metadata_path = "../artifacts/rococo-bridge.scale")]
pub mod litentry_rococo {}

pub type CONF = PolkadotConfig;

/// Relays bridge request to substrate node's runtime pallet.
pub struct SubstrateRelayer<T: Config> {
    rpc_url: String,
    key_store: SubstrateKeyStore,
    _phantom: PhantomData<T>,
}

impl<T: Config> SubstrateRelayer<T> {
    pub fn new(rpc_url: &str, key_store: SubstrateKeyStore) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            key_store,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<ChainConfig: Config> Relayer for SubstrateRelayer<ChainConfig> {
    async fn relay(&self, data: Vec<DepositRecord>) -> Result<(), ()> {
        // We only take the first data 
        let deposit_record = data[0];
        
        let call = litentry_rococo::tx()
            .bridge_transfer
            .transfer(deposit_record.destination_recipient_address, deposit_record.amount, deposit_record.resource_id);

        let api = OnlineClient::<PolkadotConfig>::from_insecure_url(&self.rpc_url)
            .await
            .map_err(|e| {
                error!("Could not connect to node: {:?}", e);
            })?;
        let secret_key_bytes = self.key_store.read().map_err(|e| {
            error!("Could not unseal key: {:?}", e);
        })?;
        let signer =
            subxt_signer::sr25519::Keypair::from_secret_key(secret_key_bytes).map_err(|e| {
                error!("Could not create secret key: {:?}", e);
            })?;

        let alice_signer = dev::alice();
        let hash = api
            .tx()
            .sign_and_submit(&call, &alice_signer, Default::default())
            .await
            .map_err(|e| {
                error!("Could not submit tx: {:?}", e);
            });

        Ok(())
    }
}
