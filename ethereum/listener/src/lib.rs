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

use crate::fetcher::Fetcher;
use crate::listener::ListenerConfig;
use alloy::primitives::Address;
use bridge_core::relay;
use bridge_core::sync_checkpoint_repository::FileCheckpointRepository;
use bridge_core::{listener::Listener, relay::Relayer};
use listener::EthereumListener;
use log::error;
use rpc_client::EthersRpcClient;
use std::collections::HashSet;
use std::str::FromStr;
use tokio::{runtime::Handle, sync::oneshot::Receiver};

mod fetcher;
pub mod listener;
mod primitives;
mod rpc_client;

/// Creates ethereum based chain listener. `finalization_gap_blocks` represents the amount of blocks
/// a listener will wait before it treat block as finalized. For example if `finalization_gap_blocks`
/// is set to 6 then listener will process block after receiving block 7, `7-1 = 6`
#[allow(clippy::result_unit_err)]
pub fn create_listener(
    id: &str,
    handle: Handle,
    config: &ListenerConfig,
    relays: Box<dyn Relayer>,
    finalization_gap_blocks: u64,
    stop_signal: Receiver<()>,
) -> Result<EthereumListener<EthersRpcClient, FileCheckpointRepository>, ()> {
    let client = EthersRpcClient::new(&config.node_rpc_url).map_err(|e| {
        error!("Could not connect to rpc: {:?}", e);
    })?;

    let last_processed_log_repository = FileCheckpointRepository::new(&format!("data/{}_last_log.bin", id));

    // TODO: Values should be receieved via CLAP instead of hardcoding
    let fetcher: Fetcher<EthersRpcClient> = Fetcher::new(
        finalization_gap_blocks,
        client,
        HashSet::from([Address::from_str(&config.bridge_contract_address).unwrap()]),
    );

    let ethereum_listener: EthereumListener<EthersRpcClient, FileCheckpointRepository> = Listener::new(
        id,
        handle,
        fetcher,
        relay::Relay::Single(relays),
        stop_signal,
        last_processed_log_repository,
        config.start_block,
    )
    .map_err(|e| error!("Error creating {} listener: {:?}", id, e))?;

    Ok(ethereum_listener)
}
