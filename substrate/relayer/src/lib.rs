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
use bridge_core::relay::{RelayError, Relayer};
use log::*;
use serde::Deserialize;
#[cfg(test)]
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use subxt::ext::subxt_core::tx::payload::StaticPayload;
use subxt::tx::Payload;
use subxt::utils::AccountId32;
use subxt::{Config, OnlineClient, PolkadotConfig};
use subxt_signer::bip39::serde;
use tokio::sync::Mutex;

pub mod key_store;

// Generate an interface that we can use from the node's metadata.
#[subxt::subxt(runtime_metadata_path = "../artifacts/paseo.scale")]
pub mod paseo {}

#[subxt::subxt(runtime_metadata_path = "../artifacts/heima.scale")]
pub mod heima {}

#[subxt::subxt(runtime_metadata_path = "../artifacts/local.scale")]
pub mod local {}

pub type CONF = PolkadotConfig;

#[derive(Deserialize)]
#[cfg_attr(test, derive(Serialize))]
pub struct RelayerConfig {
    pub ws_rpc_endpoint: String,
    pub chain: String,
}

/// Relays bridge request to substrate node's OmniBridge pallet.
pub struct SubstrateRelayer<T: Config, PRCF: PayOutRequestCallFactory> {
    rpc_url: String,
    key_store: SubstrateKeyStore,
    payout_request_call_factory: PRCF,
    destination_id: String,
    relay_lock: Mutex<()>,
    _phantom: PhantomData<T>,
}

pub fn create_from_config<T: Config>(
    keystore_dir: String,
    config_relayers: &[bridge_core::config::Relayer],
) -> HashMap<String, Arc<Box<dyn Relayer<String>>>> {
    let mut relayers: HashMap<String, Arc<Box<dyn Relayer<String>>>> = HashMap::new();
    for relayer_config in config_relayers.iter().filter(|r| r.relayer_type == "substrate") {
        let key_store = SubstrateKeyStore::new(format!("{}/{}.bin", keystore_dir.clone(), relayer_config.id));

        let signer = subxt_signer::sr25519::Keypair::from_secret_key(key_store.read().unwrap())
            .map_err(|e| {
                error!("Could not create secret key: {:?}", e);
            })
            .unwrap();

        info!("Substrate relayer address: {}", signer.public_key().to_account_id());

        let substrate_relayer_config: RelayerConfig = relayer_config.to_specific_config();

        match substrate_relayer_config.chain.as_str() {
            "local" => {
                let payout_request_call_factory = LocalPayOutRequestCallFactory {};
                let relayer: SubstrateRelayer<T, LocalPayOutRequestCallFactory> = SubstrateRelayer::new(
                    &substrate_relayer_config.ws_rpc_endpoint,
                    key_store,
                    relayer_config.destination_id.clone(),
                    payout_request_call_factory,
                );
                relayers.insert(relayer_config.id.to_string(), Arc::new(Box::new(relayer)));
            },
            "paseo" => {
                let payout_request_call_factory = PaseoPayOutRequestCallFactory {};
                let relayer: SubstrateRelayer<T, PaseoPayOutRequestCallFactory> = SubstrateRelayer::new(
                    &substrate_relayer_config.ws_rpc_endpoint,
                    key_store,
                    relayer_config.destination_id.clone(),
                    payout_request_call_factory,
                );
                relayers.insert(relayer_config.id.to_string(), Arc::new(Box::new(relayer)));
            },
            "heima" => {
                let payout_request_call_factory = HeimaPayOutRequestCallFactory {};
                let relayer: SubstrateRelayer<T, HeimaPayOutRequestCallFactory> = SubstrateRelayer::new(
                    &substrate_relayer_config.ws_rpc_endpoint,
                    key_store,
                    relayer_config.destination_id.clone(),
                    payout_request_call_factory,
                );
                relayers.insert(relayer_config.id.to_string(), Arc::new(Box::new(relayer)));
            },
            _ => panic!("Unknown chain in relayer config"),
        }
    }

    relayers
}

pub trait PayOutRequestCallFactory: Send + Sync {
    type PayOutRequestCallType: Debug + Payload + Send + Sync;

    fn create(
        &self,
        amount: u128,
        nonce: u64,
        resource_id: [u8; 32],
        account: AccountId32,
        chain_id: u32,
    ) -> Self::PayOutRequestCallType;
}

pub struct LocalPayOutRequestCallFactory {}

impl PayOutRequestCallFactory for LocalPayOutRequestCallFactory {
    type PayOutRequestCallType = StaticPayload<local::omni_bridge::calls::types::RequestPayOut>;

    fn create(
        &self,
        amount: u128,
        nonce: u64,
        resource_id: [u8; 32],
        account: AccountId32,
        chain_id: u32,
    ) -> Self::PayOutRequestCallType {
        let request = local::runtime_types::pallet_omni_bridge::PayOutRequest {
            source_chain: crate::local::runtime_types::core_primitives::omni::chain::ChainType::Ethereum(chain_id),
            nonce,
            resource_id,
            dest_account: account,
            amount,
        };
        local::tx().omni_bridge().request_pay_out(request, true)
    }
}

pub struct PaseoPayOutRequestCallFactory {}

impl PayOutRequestCallFactory for PaseoPayOutRequestCallFactory {
    type PayOutRequestCallType = StaticPayload<paseo::omni_bridge::calls::types::RequestPayOut>;

    fn create(
        &self,
        amount: u128,
        nonce: u64,
        resource_id: [u8; 32],
        account: AccountId32,
        chain_id: u32,
    ) -> Self::PayOutRequestCallType {
        let request = paseo::runtime_types::pallet_omni_bridge::PayOutRequest {
            source_chain: crate::paseo::runtime_types::core_primitives::omni::chain::ChainType::Ethereum(chain_id),
            nonce,
            resource_id,
            dest_account: account,
            amount,
        };
        paseo::tx().omni_bridge().request_pay_out(request, true)
    }
}

pub struct HeimaPayOutRequestCallFactory {}

impl PayOutRequestCallFactory for HeimaPayOutRequestCallFactory {
    type PayOutRequestCallType = StaticPayload<heima::omni_bridge::calls::types::RequestPayOut>;

    fn create(
        &self,
        amount: u128,
        nonce: u64,
        resource_id: [u8; 32],
        account: AccountId32,
        chain_id: u32,
    ) -> Self::PayOutRequestCallType {
        let request = heima::runtime_types::pallet_omni_bridge::PayOutRequest {
            source_chain: crate::heima::runtime_types::core_primitives::omni::chain::ChainType::Ethereum(chain_id),
            nonce,
            resource_id,
            dest_account: account,
            amount,
        };
        heima::tx().omni_bridge().request_pay_out(request, true)
    }
}

impl<T: Config, PRCF: PayOutRequestCallFactory> SubstrateRelayer<T, PRCF> {
    pub fn new(
        rpc_url: &str,
        key_store: SubstrateKeyStore,
        destination_id: String,
        payout_request_call_factory: PRCF,
    ) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            key_store,
            destination_id,
            payout_request_call_factory,
            relay_lock: Mutex::new(()),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<ChainConfig: Config, PRCF: PayOutRequestCallFactory> Relayer<String> for SubstrateRelayer<ChainConfig, PRCF> {
    async fn relay(
        &self,
        amount: u128,
        nonce: u64,
        resource_id: &[u8; 32],
        _data: &[u8],
        chain_id: u32,
    ) -> Result<(), RelayError> {
        let account_bytes: [u8; 32] = _data[64..96].try_into().unwrap();
        let account: AccountId32 = AccountId32::from(account_bytes);
        debug!("Relaying amount: {} with nonce: {} to account: {:?}", amount, nonce, account);
        let call = self
            .payout_request_call_factory
            .create(amount, nonce, resource_id.to_owned(), account, chain_id);
        log::debug!("Submitting PayOutRequest extrinsic: {:?}", call);

        let api = OnlineClient::<PolkadotConfig>::from_insecure_url(&self.rpc_url)
            .await
            .map_err(|e| {
                error!("Could not connect to node: {:?}", e);
                RelayError::TransportError
            })?;
        let secret_key_bytes = self.key_store.read().map_err(|e| {
            error!("Could not unseal key: {:?}", e);
            RelayError::Other
        })?;
        let signer = subxt_signer::sr25519::Keypair::from_secret_key(secret_key_bytes).map_err(|e| {
            error!("Could not create secret key: {:?}", e);
            RelayError::Other
        })?;

        // lets aquire lock here so no two tx's are pending for finalization, this will ensure that subxt logic will always get correct nonce from chain
        // alternative solution is to handle nonces on our side so we can submit txs in parallel (with different nonces)
        let _lock = self.relay_lock.lock().await;

        let hash = api
            .tx()
            .sign_and_submit_then_watch(&call, &signer, Default::default())
            .await
            .map_err(|e| {
                error!("Could not submit tx: {:?}", e);
                RelayError::TransportError
            })?
            .wait_for_finalized_success()
            .await
            .map_err(|e| {
                error!("Transaction not finalized: {:?}", e);
                RelayError::Other
            })?;

        debug!("Relayed pay out request with hash: {:?}", hash);

        Ok(())
    }

    fn destination_id(&self) -> String {
        self.destination_id.clone()
    }
}
