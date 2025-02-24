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

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
use mockall::automock;

/// Represents relayers assigned to `Listener` instance. For example PayIns from different smart contracts deployed on same EVM
/// network may be relayed to different destination chains. Strictly speaking there is a correlation between event emitter and relayer.
pub enum Relay<DestinationId> {
    Single(Arc<Box<dyn Relayer<DestinationId>>>),
    Multi(HashMap<DestinationId, Arc<Box<dyn Relayer<DestinationId>>>>),
}

/// Used to relay bridging request to destination chain
#[async_trait]
#[cfg_attr(test, automock)]
pub trait Relayer<DestinationId: Send + Sync>: Send + Sync {
    // todo: chain id should represent chain_type + index instead of just index
    async fn relay(
        &self,
        amount: u128,
        nonce: u64,
        resource_id: [u8; 32],
        data: Vec<u8>,
        chain_id: u32,
    ) -> Result<(), RelayError>;
    fn destination_id(&self) -> DestinationId;
}

pub enum RelayError {
    TransportError,
    Other,
}
