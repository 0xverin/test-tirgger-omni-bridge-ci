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

use crate::listener::DepositRecord;
use async_trait::async_trait;

/// Returns the last finalized block number
#[async_trait]
pub trait LastFinalizedBlockNumFetcher {
    async fn get_last_finalized_block_num(&mut self) -> Result<Option<u64>, ()>;
}

/// Returns all PayIn events emitted on given chain
/// SourceId can be used if there are more event emitters - for example smart contracts on EVM based chain
/// This means that if there are two or more smart contracts deployed on the same chain, it should be possible to
/// fetch events from all of them together.
#[async_trait]
pub trait BlockPayInEventsFetcher<Id: Clone, EventSourceId: Clone> {
    async fn get_block_pay_in_events(
        &mut self,
        block_num: u64,
    ) -> Result<Vec<DepositRecord>, ()>;
}
