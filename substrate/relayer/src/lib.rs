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
use bridge_core::config::BridgeConfig;
use bridge_core::key_store::KeyStore;
use bridge_core::relay::Relayer;
use log::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::marker::PhantomData;
use subxt::utils::AccountId32;
use subxt::{Config, OnlineClient, PolkadotConfig};
use subxt_signer::bip39::serde;

pub mod key_store;

// Generate an interface that we can use from the node's metadata.
#[subxt::subxt(runtime_metadata_path = "../artifacts/rococo-bridge.scale")]
pub mod litentry_rococo {}

pub type CONF = PolkadotConfig;

#[derive(Deserialize)]
pub struct RelayerConfig {
    pub ws_rpc_endpoint: String,
}

/// Relays bridge request to substrate node's OmniBridge pallet.
pub struct SubstrateRelayer<T: Config> {
    rpc_url: String,
    key_store: SubstrateKeyStore,
    _phantom: PhantomData<T>,
}

pub fn create_from_config<T: Config>(keystore_dir: String, config: &BridgeConfig) -> HashMap<String, Box<dyn Relayer>> {
    let mut relayers: HashMap<String, Box<dyn Relayer>> = HashMap::new();
    for relayer_config in config.relayers.iter().filter(|r| r.relayer_type == "substrate") {
        let key_store = SubstrateKeyStore::new(format!("{}/{}.bin", keystore_dir.clone(), relayer_config.id));

        let signer = subxt_signer::sr25519::Keypair::from_secret_key(key_store.read().unwrap())
            .map_err(|e| {
                error!("Could not create secret key: {:?}", e);
            })
            .unwrap();

        info!("Substrate relayer address: {}", signer.public_key().to_account_id());

        let substrate_relayer_config: RelayerConfig = relayer_config.to_specific_config();
        let relayer: SubstrateRelayer<T> = SubstrateRelayer::new(&substrate_relayer_config.ws_rpc_endpoint, key_store);
        relayers.insert(relayer_config.id.to_string(), Box::new(relayer));
    }

    relayers
}

impl<T: Config> SubstrateRelayer<T> {
    pub fn new(rpc_url: &str, key_store: SubstrateKeyStore) -> Self {
        Self { rpc_url: rpc_url.to_string(), key_store, _phantom: PhantomData }
    }
}

#[async_trait]
impl<ChainConfig: Config> Relayer for SubstrateRelayer<ChainConfig> {
    async fn relay(&self, amount: u128, nonce: u64, resource_id: [u8; 32], _data: Vec<u8>) -> Result<(), ()> {
        let account_bytes: [u8; 32] = _data[64..96].try_into().unwrap();
        let account: AccountId32 = AccountId32::from(account_bytes);
        debug!("Relaying amount: {} with nonce: {} to account: {:?}", amount, nonce, account);

        let request = litentry_rococo::runtime_types::pallet_omni_bridge::PayOutRequest {
            //todo: should not be hardcoded
            source_chain: litentry_rococo::runtime_types::pallet_omni_bridge::ChainType::Ethereum(0),
            nonce,
            resource_id,
            dest_account: account,
            amount,
        };

        let call = litentry_rococo::tx().omni_bridge().request_pay_out(request, true);

        log::debug!("Submitting PayOutRequest extrinsic: {:?}", call);

        let api = OnlineClient::<PolkadotConfig>::from_insecure_url(&self.rpc_url)
            .await
            .map_err(|e| {
                error!("Could not connect to node: {:?}", e);
            })?;
        let secret_key_bytes = self.key_store.read().map_err(|e| {
            error!("Could not unseal key: {:?}", e);
        })?;
        let signer = subxt_signer::sr25519::Keypair::from_secret_key(secret_key_bytes).map_err(|e| {
            error!("Could not create secret key: {:?}", e);
        })?;

        let hash = api.tx().sign_and_submit(&call, &signer, Default::default()).await.map_err(|e| {
            error!("Could not submit tx: {:?}", e);
        });

        debug!("Relayed pay out request with hash: {:?}", hash);

        Ok(())
    }
}
