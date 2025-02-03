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

mod fetcher;
pub mod listener;
mod primitives;
mod rpc_client;

use crate::fetcher::Fetcher;
use crate::listener::{ListenerConfig, SubstrateListener};
use crate::rpc_client::{RpcClient, RpcClientFactory};
use bridge_core::listener::Listener;
use bridge_core::relay::{Relay, Relayer};
use bridge_core::sync_checkpoint_repository::FileCheckpointRepository;
use scale_encode::EncodeAsType;
use subxt::config::signed_extensions;
use subxt::events::StaticEvent;
use subxt::Config;
use tokio::runtime::Handle;
use tokio::sync::oneshot::Receiver;

// Generate an interface that we can use from the node's metadata.
#[subxt::subxt(runtime_metadata_path = "../artifacts/paseo.scale")]
pub mod paseo {}

#[subxt::subxt(runtime_metadata_path = "../artifacts/heima.scale")]
pub mod heima {}

#[subxt::subxt(runtime_metadata_path = "../artifacts/local.scale")]
pub mod local {}

// We don't need to construct this at runtime,
// so an empty enum is appropriate:
#[derive(EncodeAsType)]
pub enum CustomConfig {}

impl Config for CustomConfig {
    type Hash = subxt::utils::H256;
    type AccountId = subxt::utils::AccountId32;
    type Address = subxt::utils::MultiAddress<Self::AccountId, ()>;
    type Signature = subxt::utils::MultiSignature;
    type Hasher = subxt::config::substrate::BlakeTwo256;
    type Header = subxt::config::substrate::SubstrateHeader<u32, Self::Hasher>;
    type ExtrinsicParams = signed_extensions::AnyOf<
        Self,
        (
            // Load in the existing signed extensions we're interested in
            // (if the extension isn't actually needed it'll just be ignored):
            signed_extensions::CheckSpecVersion,
            signed_extensions::CheckTxVersion,
            signed_extensions::CheckNonce,
            signed_extensions::CheckGenesis<Self>,
            signed_extensions::CheckMortality<Self>,
            signed_extensions::ChargeAssetTxPayment<Self>,
            signed_extensions::ChargeTransactionPayment,
            signed_extensions::CheckMetadataHash,
        ),
    >;
    type AssetId = u32;
}

/// Creates local substrate based chain listener.
pub async fn create_local_listener<ChainConfig: Config>(
    id: &str,
    handle: Handle,
    config: &ListenerConfig,
    start_block: u64,
    relayer: Box<dyn Relayer>,
    stop_signal: Receiver<()>,
) -> Result<
    SubstrateListener<
        RpcClient<ChainConfig, LocalPaidInEvent>,
        RpcClientFactory<ChainConfig>,
        FileCheckpointRepository,
    >,
    (),
> {
    let client_factory: RpcClientFactory<ChainConfig> = RpcClientFactory::new(&config.ws_rpc_endpoint);

    let fetcher = Fetcher::new(client_factory);
    let last_processed_log_repository = FileCheckpointRepository::new(&format!("data/{}_last_log.bin", id));

    Listener::new(id, handle, fetcher, Relay::Single(relayer), stop_signal, last_processed_log_repository, start_block)
}

/// Creates Paseo chain listener.
pub async fn create_paseo_listener<ChainConfig: Config>(
    id: &str,
    handle: Handle,
    config: &ListenerConfig,
    start_block: u64,
    relayer: Box<dyn Relayer>,
    stop_signal: Receiver<()>,
) -> Result<
    SubstrateListener<
        RpcClient<ChainConfig, PaseoPaidInEvent>,
        RpcClientFactory<ChainConfig>,
        FileCheckpointRepository,
    >,
    (),
> {
    let client_factory: RpcClientFactory<ChainConfig> = RpcClientFactory::new(&config.ws_rpc_endpoint);

    let fetcher = Fetcher::new(client_factory);
    let last_processed_log_repository = FileCheckpointRepository::new(&format!("data/{}_last_log.bin", id));

    Listener::new(id, handle, fetcher, Relay::Single(relayer), stop_signal, last_processed_log_repository, start_block)
}

/// Creates Heima chain listener.
pub async fn create_heima_listener<ChainConfig: Config>(
    id: &str,
    handle: Handle,
    config: &ListenerConfig,
    start_block: u64,
    relayer: Box<dyn Relayer>,
    stop_signal: Receiver<()>,
) -> Result<
    SubstrateListener<
        RpcClient<ChainConfig, HeimaPaidInEvent>,
        RpcClientFactory<ChainConfig>,
        FileCheckpointRepository,
    >,
    (),
> {
    let client_factory: RpcClientFactory<ChainConfig> = RpcClientFactory::new(&config.ws_rpc_endpoint);

    let fetcher = Fetcher::new(client_factory);
    let last_processed_log_repository = FileCheckpointRepository::new(&format!("data/{}_last_log.bin", id));

    Listener::new(id, handle, fetcher, Relay::Single(relayer), stop_signal, last_processed_log_repository, start_block)
}

pub trait PalletPaidInEvent: Send {
    type MetadataType: StaticEvent;

    fn wrap(raw: Self::MetadataType) -> Self;

    fn amount(&self) -> u128;
    fn resource_id(&self) -> [u8; 32];
    fn dest_account(&self) -> Vec<u8>;
    fn nonce(&self) -> u64;
}

pub struct HeimaPaidInEvent {
    raw: heima::omni_bridge::events::PaidIn,
}

impl PalletPaidInEvent for HeimaPaidInEvent {
    type MetadataType = heima::omni_bridge::events::PaidIn;

    fn wrap(raw: Self::MetadataType) -> Self {
        Self { raw }
    }

    fn amount(&self) -> u128 {
        self.raw.amount
    }

    fn resource_id(&self) -> [u8; 32] {
        self.raw.resource_id
    }

    fn dest_account(&self) -> Vec<u8> {
        self.raw.dest_account.clone()
    }

    fn nonce(&self) -> u64 {
        self.raw.nonce
    }
}

pub struct LocalPaidInEvent {
    raw: local::omni_bridge::events::PaidIn,
}

impl PalletPaidInEvent for LocalPaidInEvent {
    type MetadataType = local::omni_bridge::events::PaidIn;

    fn wrap(raw: Self::MetadataType) -> Self {
        Self { raw }
    }

    fn amount(&self) -> u128 {
        self.raw.amount
    }

    fn resource_id(&self) -> [u8; 32] {
        self.raw.resource_id
    }

    fn dest_account(&self) -> Vec<u8> {
        self.raw.dest_account.clone()
    }

    fn nonce(&self) -> u64 {
        self.raw.nonce
    }
}

pub struct PaseoPaidInEvent {
    raw: paseo::omni_bridge::events::PaidIn,
}

impl PalletPaidInEvent for PaseoPaidInEvent {
    type MetadataType = paseo::omni_bridge::events::PaidIn;

    fn wrap(raw: Self::MetadataType) -> Self {
        Self { raw }
    }

    fn amount(&self) -> u128 {
        self.raw.amount
    }

    fn resource_id(&self) -> [u8; 32] {
        self.raw.resource_id
    }

    fn dest_account(&self) -> Vec<u8> {
        self.raw.dest_account.clone()
    }

    fn nonce(&self) -> u64 {
        self.raw.nonce
    }
}
