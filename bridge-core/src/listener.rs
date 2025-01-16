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

use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::{hash::Hash, marker::PhantomData, thread::sleep, time::Duration};
use tokio::{runtime::Handle, sync::oneshot::Receiver};

use crate::config::BridgeConfig;
use crate::fetcher::{BlockPayInEventsFetcher, LastFinalizedBlockNumFetcher};
use crate::{
    relay::Relay,
    sync_checkpoint_repository::{Checkpoint, CheckpointRepository},
};

/// Represents `PayIn` event emitted on one side of the bridge.
#[derive(Clone, Debug, PartialEq)]
pub struct PayIn<Id: Clone, EventSourceId: Clone> {
    id: Id,
    maybe_event_source: Option<EventSourceId>,
    amount: u128,
    nonce: u64,
    resource_id: [u8; 32],
    data: Vec<u8>,
}

impl<Id: Clone, EventSourceId: Clone> PayIn<Id, EventSourceId> {
    pub fn new(
        id: Id,
        maybe_event_source: Option<EventSourceId>,
        amount: u128,
        nonce: u64,
        resource_id: [u8; 32],
        data: Vec<u8>,
    ) -> Self {
        Self { id, maybe_event_source, amount, nonce, resource_id, data }
    }
}

pub struct StartBlock {
    pub listener_id: String,
    pub block_num: u64,
}

impl TryFrom<&String> for StartBlock {
    type Error = ();

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        let values: Vec<&str> = value.split(":").collect();
        let block_num = (values.get(1).ok_or(())?).parse::<u64>().map_err(|_| ())?;
        Ok(StartBlock { listener_id: values.first().unwrap().to_string(), block_num })
    }
}

pub struct ListenerContext<T> {
    pub id: String,
    pub config: T,
    pub start_block: u64,
    pub relayers: Vec<Box<dyn crate::relay::Relayer>>,
}

pub fn prepare_listener_context<T: DeserializeOwned>(
    config: &BridgeConfig,
    listener_type: &str,
    relayers: &mut HashMap<String, HashMap<String, Box<dyn crate::relay::Relayer>>>,
    start_blocks: &HashMap<String, u64>,
) -> Vec<ListenerContext<T>> {
    let mut components = vec![];
    for listener_config in config.listeners.iter().filter(|l| l.listener_type == listener_type) {
        let ethereum_listener_config: T = listener_config.to_specific_config();
        let mut listener_relayers: Vec<Box<dyn crate::relay::Relayer>> = vec![];

        for relayer_id in listener_config.relayers.iter() {
            for relayers in relayers.values_mut() {
                if let Some(relayer) = relayers.remove(relayer_id) {
                    listener_relayers.push(relayer)
                }
            }
        }

        let start_block = *start_blocks.get(&listener_config.id).unwrap_or(&0);

        components.push(ListenerContext {
            id: listener_config.id.clone(),
            config: ethereum_listener_config,
            start_block,
            relayers: listener_relayers,
        });
    }
    components
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
    start_block: u64,
    _phantom: PhantomData<(Checkpoint, PayInEventId)>,
}

#[allow(clippy::result_unit_err)]
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
        start_block: u64,
    ) -> Result<Self, ()> {
        Ok(Self {
            id: id.to_string(),
            handle,
            fetcher,
            relay,
            stop_signal,
            checkpoint_repository: last_processed_log_repository,
            start_block,
            _phantom: PhantomData,
        })
    }

    /// Start syncing. It's a long-running blocking operation - should be started in dedicated thread.
    pub fn sync(&mut self) {
        log::info!("Starting {} network sync, start block: {}", self.id, self.start_block);
        let mut block_number_to_sync =
            if let Some(ref checkpoint) = self.checkpoint_repository.get().expect("Could not read checkpoint") {
                let last_block_num = checkpoint.get_block_num();

                // Ensure `start_block` overrides only if it's valid
                if self.start_block > last_block_num {
                    self.start_block
                } else if checkpoint.just_block_num() {
                    // Start syncing from the next block as we processed the previous one fully
                    last_block_num + 1
                } else {
                    // Reprocess the last block if interrupted
                    last_block_num
                }
            } else {
                // Default to start_block if no checkpoint exists
                self.start_block
            };
        log::debug!("Starting sync from {:?}", block_number_to_sync);

        'main: loop {
            log::debug!("Starting syncing block: {}", block_number_to_sync);
            if self.stop_signal.try_recv().is_ok() {
                break;
            }

            let maybe_last_finalized_block = match self.handle.block_on(self.fetcher.get_last_finalized_block_num()) {
                Ok(maybe_block) => maybe_block,
                Err(_) => {
                    log::debug!("Could not get last finalized block number");
                    sleep(Duration::from_secs(1));
                    continue;
                },
            };

            let last_finalized_block = match maybe_last_finalized_block {
                Some(v) => v,
                None => {
                    log::debug!("Waiting for finalized block, block to sync {}", block_number_to_sync);
                    sleep(Duration::from_secs(1));
                    continue;
                },
            };

            log::trace!("Last finalized block: {}, block to sync {}", last_finalized_block, block_number_to_sync);

            //we know there are more block waiting for sync so let's skip sleep
            let fast = match last_finalized_block.checked_sub(block_number_to_sync) {
                Some(v) => v > 1,
                None => false,
            };

            if last_finalized_block >= block_number_to_sync {
                if let Ok(events) = self.handle.block_on(self.fetcher.get_block_pay_in_events(block_number_to_sync)) {
                    for event in events {
                        let maybe_relayer = match self.relay {
                            Relay::Single(ref relay) => Some(relay),
                            Relay::Multi(ref relayers) => {
                                if let Some(event_source_id) = event.maybe_event_source {
                                    relayers.get(&event_source_id)
                                } else {
                                    None
                                }
                            },
                        };
                        if let Some(relayer) = maybe_relayer {
                            if let Some(ref checkpoint) =
                                self.checkpoint_repository.get().expect("Could not read checkpoint")
                            {
                                if checkpoint.lt(&event.id.clone().into()) {
                                    log::info!("Relaying");
                                    if self
                                        .handle
                                        .block_on(relayer.relay(
                                            event.amount,
                                            event.nonce,
                                            event.resource_id,
                                            event.data,
                                        ))
                                        .is_err()
                                    {
                                        log::info!("Could not relay");
                                        sleep(Duration::from_secs(1));
                                        continue 'main;
                                    }
                                } else {
                                    log::debug!("Skipping event");
                                }
                            } else if self
                                .handle
                                .block_on(relayer.relay(event.amount, event.nonce, event.resource_id, event.data))
                                .is_err()
                            {
                                log::info!("Could not relay");
                                sleep(Duration::from_secs(1));
                                continue 'main;
                            }
                        }
                        self.checkpoint_repository
                            .save(event.id.into())
                            .expect("Could not save checkpoint");
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
