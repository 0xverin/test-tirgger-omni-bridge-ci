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
use crate::primitives::{LogId, SyncCheckpoint};
use bridge_core::listener::{Listener, PayIn};
use serde::Deserialize;

pub type PayInEventId = LogId;
pub type DestinationId = String;
pub type EthereumPayInEvent = PayIn<PayInEventId, DestinationId>;

#[derive(Deserialize)]
pub struct ListenerConfig {
    pub node_rpc_url: String,
    pub bridge_contract_address: String,
    pub finalization_gap: u64,
}

pub type EthereumListener<RpcClient, CheckpointRepository> =
    Listener<DestinationId, Fetcher<RpcClient>, SyncCheckpoint, CheckpointRepository, PayInEventId>;
