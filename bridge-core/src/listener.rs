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

use std::{hash::Hash, marker::PhantomData, thread::sleep, time::Duration};
use alloy::primitives::{U256, Address, Bytes, FixedBytes};
use std::str::FromStr;

use tokio::{runtime::Handle, sync::oneshot::Receiver};

use crate::fetcher::{BlockPayInEventsFetcher, LastFinalizedBlockNumFetcher};
use crate::{
    relay::{Relay, Relayer},
    sync_checkpoint_repository::{Checkpoint, CheckpointRepository},
};

/// Represents `PayIn` event emitted on one side of the bridge.
#[derive(Clone, Debug, PartialEq)]
pub struct PayIn<Id: Clone, EventSourceId: Clone> {
    id: Id,
    maybe_event_source: Option<EventSourceId>,
    amount: u128,
    data: Vec<u8>,
}

impl<Id: Clone, EventSourceId: Clone> PayIn<Id, EventSourceId> {
    pub fn new(
        id: Id,
        maybe_event_source: Option<EventSourceId>,
        amount: u128,
        data: Vec<u8>,
    ) -> Self {
        Self {
            id,
            maybe_event_source,
            amount,
            data,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DepositRecord {
    pub token_address: Address,                // Solidity "address" -> Alloy "Address"
    pub destination_chain_id: u8,             // Solidity "uint8" -> Rust "u8"
    pub resource_id: FixedBytes<32>,                // Solidity "bytes32" -> Fixed-size Rust array
    pub destination_recipient_address: Bytes, // Solidity "bytes" -> Rust Vec<u8>
    pub depositer: Address,                   // Solidity "address" -> Alloy "Address"
    pub amount: U256,
    pub nonce: u64                         // Solidity "uint" (uint256) -> Alloy "U256"
}


/// Core component, used to listen to source chain and relay bridge request to destination chain.
/// Requires specific implementations of:
/// `Fetcher` - used to fetch data from source chain
/// `Relayer` - used to relay bridge requests to destination chain
/// `CheckpointRepository` - used to store listener's progress
pub struct Listener<EventSourceId, Fetcher, Checkpoint, CheckpointRepository, PayInEventId> {
    id: String,
    handle: Handle,
    fetcher: Fetcher,
    relay: Relay<EventSourceId>,
    stop_signal: Receiver<()>,
    checkpoint_repository: CheckpointRepository,
    _phantom: PhantomData<(Checkpoint, PayInEventId)>,
}

impl<
        EventSourceId: Hash + Eq + Clone,
        PayInEventId: Into<CheckpointT> + Clone,
        Fetcher: LastFinalizedBlockNumFetcher + BlockPayInEventsFetcher<PayInEventId, EventSourceId>,
        CheckpointT: PartialOrd + Checkpoint + From<u64>,
        CheckpointRepositoryT: CheckpointRepository<CheckpointT>,
    > Listener<EventSourceId, Fetcher, CheckpointT, CheckpointRepositoryT, PayInEventId>
{
    pub fn new(
        id: &str,
        handle: Handle,
        fetcher: Fetcher,
        relay: Relay<EventSourceId>,
        stop_signal: Receiver<()>,
        last_processed_log_repository: CheckpointRepositoryT,
    ) -> Result<Self, ()> {
        Ok(Self {
            id: id.to_string(),
            handle,
            fetcher,
            relay,
            stop_signal,
            checkpoint_repository: last_processed_log_repository,
            _phantom: PhantomData,
        })
    }

    /// Start syncing. It's a long-running blocking operation - should be started in dedicated thread.
    pub fn sync(&mut self, start_block: u64) {
        log::info!(
            "Starting {} network sync, start block: {}",
            self.id,
            start_block
        );
        let mut block_number_to_sync = if let Some(ref checkpoint) = self
            .checkpoint_repository
            .get()
            .expect("Could not read checkpoint")
        {
            if checkpoint.just_block_num() {
                // let's start syncing from next block as we processed previous fully
                checkpoint.get_block_num() + 1
            } else {
                // block processing was interrupted, so we have to process last block again
                // but currently processed logs will be skipped
                checkpoint.get_block_num()
            }
        } else {
            start_block
        };
        log::debug!("Starting sync from {:?}", block_number_to_sync);

        'main: loop {
            log::debug!("Starting syncing block: {}", block_number_to_sync);
            if self.stop_signal.try_recv().is_ok() {
                break;
            }

            let maybe_last_finalized_block = match self
                .handle
                .block_on(self.fetcher.get_last_finalized_block_num())
            {
                Ok(maybe_block) => maybe_block,
                Err(_) => {
                    log::debug!("Could not get last finalized block number");
                    sleep(Duration::from_secs(1));
                    continue;
                }
            };

            let last_finalized_block = match maybe_last_finalized_block {
                Some(v) => v,
                None => {
                    log::debug!(
                        "Waiting for finalized block, block to sync {}",
                        block_number_to_sync
                    );
                    sleep(Duration::from_secs(1));
                    continue;
                }
            };

            log::trace!(
                "Last finalized block: {}, block to sync {}",
                last_finalized_block,
                block_number_to_sync
            );

            //we know there are more block waiting for sync so let's skip sleep
            let fast = match last_finalized_block.checked_sub(block_number_to_sync) {
                Some(v) => v > 1,
                None => false,
            };

            if last_finalized_block >= block_number_to_sync {
                if let Ok(events) = self
                    .handle
                    .block_on(self.fetcher.get_block_pay_in_events(block_number_to_sync))
                {
                    for event in events {
                        let maybe_relayer = match self.relay {
                            Relay::Single(ref relay) => Some(relay),
                            Relay::Multi(ref relayers) => {
                                // By default, For now we will not support multi
                                None
                            }
                        };
                        if let Some(relayer) = maybe_relayer {
                            if let Some(ref checkpoint) = self
                                .checkpoint_repository
                                .get()
                                .expect("Could not read checkpoint")
                            {
                                if checkpoint.lt(&event.nonce.clone().into()) {
                                    log::info!("Relaying");
                                    if let Err(e) = self.handle.block_on(relayer.relay(vec![event.clone()])) {
                                        log::info!("Could not relay");
                                        sleep(Duration::from_secs(1));
                                        continue 'main;
                                    }
                                } else {
                                    log::debug!("Skipping event");
                                }
                            } else {
                                if let Err(e) = self.handle.block_on(relayer.relay(vec![])) {
                                    log::info!("Could not relay");
                                    sleep(Duration::from_secs(1));
                                    continue 'main;
                                }
                            }
                            self.checkpoint_repository
                                .save(event.nonce.into())
                                .expect("Could not save checkpoint");
                        }
                    }
                    // we processed block completely so store new checkpoint
                    self.checkpoint_repository
                        .save(CheckpointT::from(block_number_to_sync))
                        .expect("Could not save checkpoint");
                    log::info!("Finished syncing block: {}", block_number_to_sync);
                    block_number_to_sync += 1;
                }
            }

            if !fast {
                sleep(Duration::from_secs(1))
            } else {
                log::trace!("Fast sync skipping 1s wait");
            }
        }
    }
}
