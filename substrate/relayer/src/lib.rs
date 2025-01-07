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
use bridge_core::relay::Relayer;
use log::debug;
use serde::Deserialize;
use std::collections::HashMap;
use std::marker::PhantomData;
use subxt::utils::AccountId32;
use subxt::{Config, PolkadotConfig};
use subxt_signer::bip39::serde;

pub mod key_store;

// Generate an interface that we can use from the node's metadata.
#[subxt::subxt(runtime_metadata_path = "../artifacts/metadata.scale")]
pub mod litentry_rococo {}

pub type CONF = PolkadotConfig;

#[derive(Deserialize)]
pub struct RelayerConfig {
    pub node_rpc_url: String,
}

/// Relays bridge request to substrate node's runtime pallet.
pub struct SubstrateRelayer<T: Config> {
    _rpc_url: String,
    _key_store: SubstrateKeyStore,
    _phantom: PhantomData<T>,
}

pub fn create_from_config<T: Config>(config: &BridgeConfig) -> HashMap<String, Box<dyn Relayer>> {
    let mut relayers: HashMap<String, Box<dyn Relayer>> = HashMap::new();
    for relayer_config in config
        .relayers
        .iter()
        .filter(|r| r.relayer_type == "substrate")
    {
        let key_store =
            SubstrateKeyStore::new(format!("data/{}_relayer_key.bin", relayer_config.id));
        let substrate_relayer_config: RelayerConfig = relayer_config.to_specific_config();
        let relayer: SubstrateRelayer<T> =
            SubstrateRelayer::new(&substrate_relayer_config.node_rpc_url, key_store);
        relayers.insert(relayer_config.id.to_string(), Box::new(relayer));
    }

    relayers
}

impl<T: Config> SubstrateRelayer<T> {
    pub fn new(rpc_url: &str, key_store: SubstrateKeyStore) -> Self {
        Self {
            _rpc_url: rpc_url.to_string(),
            _key_store: key_store,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<ChainConfig: Config> Relayer for SubstrateRelayer<ChainConfig> {
    async fn relay(&self, amount: u128, nonce: u64, _data: Vec<u8>) -> Result<(), ()> {
        let account_bytes: [u8; 32] = _data[64..96].try_into().unwrap();
        let account: AccountId32 = AccountId32::from(account_bytes);
        debug!(
            "Relaying amount: {} with nonce: {} to account: {:?}",
            amount, nonce, account
        );

        //parse account id

        // let (amount, rid, to, nonce) = data.get_bridge_transfer_arguments().unwrap();
        //
        // log::debug!("Submitting bridge_transfer extrinsic, amount: {:?}, to: {:?}", amount, to);
        //
        // let bridge = RuntimeCall::BridgeTransfer (
        //     Call::transfer{to, amount, rid: rid.clone()},
        // );
        //
        // let call = litentry_rococo::tx()
        //     .chain_bridge()
        //     .acknowledge_proposal(nonce, 0, rid, bridge);
        //
        // let api = OnlineClient::<PolkadotConfig>::from_insecure_url(&self.rpc_url)
        //     .await
        //     .map_err(|e| {
        //         error!("Could not connect to node: {:?}", e);
        //     })?;
        // let secret_key_bytes = self.key_store.read().map_err(|e| {
        //     error!("Could not unseal key: {:?}", e);
        // })?;
        // let signer =
        //     subxt_signer::sr25519::Keypair::from_secret_key(secret_key_bytes).map_err(|e| {
        //         error!("Could not create secret key: {:?}", e);
        //     })?;
        //
        // let alice_signer = dev::alice();
        //
        // // TODO: This should be submit and watch
        // let hash = api
        //     .tx()
        //     .sign_and_submit(&call, &alice_signer, Default::default())
        //     .await
        //     .map_err(|e| {
        //         error!("Could not submit tx: {:?}", e);
        //     });
        //
        // // Note: Hash doesn't guranttee success of extrinsic
        // if let Ok(hash_of) = hash {
        //     log::debug!("Submtted extrinsic succesfully: {:?}", hash_of);
        // } else {
        //     log::error!("Failed to submit extrinsics succesfully");
        // }

        Ok(())
    }
}
