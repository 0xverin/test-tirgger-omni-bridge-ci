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

use metrics::{describe_gauge, gauge};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::{hash::Hash, marker::PhantomData, thread::sleep, time::Duration};
use tokio::{runtime::Handle, sync::oneshot::Receiver};

use crate::config::BridgeConfig;
use crate::fetcher::{BlockPayInEventsFetcher, LastFinalizedBlockNumFetcher};
use crate::relay::RelayError;
use crate::{
    relay::Relay,
    sync_checkpoint_repository::{Checkpoint, CheckpointRepository},
};

/// Represents `PayIn` event emitted on one side of the bridge.
#[derive(Clone, Debug, PartialEq)]
pub struct PayIn<Id: Clone, DestinationId: Clone> {
    id: Id,
    maybe_destination_id: Option<DestinationId>,
    amount: u128,
    nonce: u64,
    resource_id: [u8; 32],
    data: Vec<u8>,
}

impl<Id: Clone, DestinationId: Clone> PayIn<Id, DestinationId> {
    pub fn new(
        id: Id,
        maybe_destination_id: Option<DestinationId>,
        amount: u128,
        nonce: u64,
        resource_id: [u8; 32],
        data: Vec<u8>,
    ) -> Self {
        Self { id, maybe_destination_id, amount, nonce, resource_id, data }
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
    pub chain_id: u32,
    pub relayers: HashMap<String, Arc<Box<dyn crate::relay::Relayer<String>>>>,
}

#[allow(clippy::type_complexity)]
pub fn prepare_listener_context<T: DeserializeOwned>(
    config: &BridgeConfig,
    listener_type: &str,
    relayers: &HashMap<String, HashMap<String, Arc<Box<dyn crate::relay::Relayer<String>>>>>,
    start_blocks: &HashMap<String, u64>,
) -> Vec<ListenerContext<T>> {
    let mut components = vec![];
    for listener_config in config.listeners.iter().filter(|l| l.listener_type == listener_type) {
        let ethereum_listener_config: T = listener_config.to_specific_config();
        let mut listener_relayers: HashMap<String, Arc<Box<dyn crate::relay::Relayer<String>>>> = HashMap::new();
        for relayer_id in listener_config.relayers.iter() {
            for relayers in relayers.values() {
                if let Some(relayer) = relayers.get(relayer_id) {
                    listener_relayers.insert(relayer.destination_id(), relayer.clone());
                }
            }
        }

        let start_block = *start_blocks.get(&listener_config.id).unwrap_or(&0);

        components.push(ListenerContext {
            id: listener_config.id.clone(),
            config: ethereum_listener_config,
            start_block,
            chain_id: listener_config.chain_id,
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
pub struct Listener<DestinationId, Fetcher, Checkpoint, CheckpointRepository, PayInEventId> {
    id: String,
    handle: Handle,
    fetcher: Fetcher,
    relay: Relay<DestinationId>,
    stop_signal: Receiver<()>,
    checkpoint_repository: CheckpointRepository,
    start_block: u64,
    chain_id: u32,
    _phantom: PhantomData<(Checkpoint, PayInEventId)>,
}

#[allow(clippy::result_unit_err, clippy::too_many_arguments)]
impl<
        DestinationId: Hash + Eq + Clone + Debug + Send + Sync,
        PayInEventId: Into<CheckpointT> + Clone,
        Fetcher: LastFinalizedBlockNumFetcher + BlockPayInEventsFetcher<PayInEventId, DestinationId>,
        CheckpointT: PartialOrd + Checkpoint + From<u64>,
        CheckpointRepositoryT: CheckpointRepository<CheckpointT>,
    > Listener<DestinationId, Fetcher, CheckpointT, CheckpointRepositoryT, PayInEventId>
{
    pub fn new(
        id: &str,
        handle: Handle,
        fetcher: Fetcher,
        relay: Relay<DestinationId>,
        stop_signal: Receiver<()>,
        last_processed_log_repository: CheckpointRepositoryT,
        start_block: u64,
        chain_id: u32,
    ) -> Result<Self, ()> {
        describe_gauge!(synced_block_gauge_name(id), "Last synced block");
        Ok(Self {
            id: id.to_string(),
            handle,
            fetcher,
            relay,
            stop_signal,
            checkpoint_repository: last_processed_log_repository,
            start_block,
            chain_id,
            _phantom: PhantomData,
        })
    }

    /// Start syncing. It's a long-running blocking operation - should be started in dedicated thread.
    pub fn sync(&mut self) -> Result<(), ()> {
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
                return Ok(());
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
                match self.handle.block_on(self.fetcher.get_block_pay_in_events(block_number_to_sync)) {
                    Ok(events) => {
                        for event in events {
                            let maybe_relayer = match self.relay {
                                Relay::Single(ref relay) => Some(relay),
                                Relay::Multi(ref relayers) => {
                                    if let Some(destination_id) = event.maybe_destination_id {
                                        relayers.get(&destination_id)
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
                                        let mut attempt = 1;
                                        'relay: loop {
                                            log::info!("Relaying attempt: {}", attempt);

                                            if attempt > 10 {
                                                log::error!("Exceeded maximum number of relaying attempts");
                                                return Err(());
                                            }

                                            match self.handle.block_on(relayer.relay(
                                                event.amount,
                                                event.nonce,
                                                &event.resource_id,
                                                &event.data,
                                                self.chain_id,
                                            )) {
                                                Err(RelayError::TransportError) => {
                                                    log::info!(
                                                        "Could not relay due to TransportError, will try again..."
                                                    );
                                                    sleep(Duration::from_secs(1));
                                                    attempt += 1;
                                                    continue 'relay;
                                                },
                                                Err(RelayError::Other) => {
                                                    log::error!("Unexpected error occurred during relaying");
                                                    return Err(());
                                                },
                                                Err(RelayError::WatchError) => {
                                                    // retry the same event again
                                                    attempt += 1;
                                                    continue 'relay;
                                                },
                                                Err(RelayError::AlreadyRelayed) => {
                                                    log::error!("Already relayed");
                                                    break 'relay;
                                                },
                                                _ => break 'relay,
                                            }
                                        }
                                    } else {
                                        log::debug!("Skipping event");
                                    }
                                } else {
                                    let mut attempt = 1;
                                    'relay: loop {
                                        log::info!("Relaying attempt: {}", attempt);

                                        if attempt > 10 {
                                            log::error!("Exceeded maximum number of relaying attempts");
                                            return Err(());
                                        }

                                        match self.handle.block_on(relayer.relay(
                                            event.amount,
                                            event.nonce,
                                            &event.resource_id,
                                            &event.data,
                                            self.chain_id,
                                        )) {
                                            Err(RelayError::TransportError) => {
                                                log::info!("Could not relay due to TransportError, will try again...");
                                                sleep(Duration::from_secs(1));
                                                attempt += 1;
                                                continue 'relay;
                                            },
                                            Err(RelayError::Other) => {
                                                log::error!("Unexpected error occurred during relaying");
                                                return Err(());
                                            },
                                            Err(RelayError::WatchError) => {
                                                // retry the same event again
                                                attempt += 1;
                                                continue 'relay;
                                            },
                                            Err(RelayError::AlreadyRelayed) => {
                                                log::error!("Already relayed");
                                                break 'relay;
                                            },
                                            _ => break 'relay,
                                        }
                                    }
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
                        gauge!(synced_block_gauge_name(&self.id)).set(block_number_to_sync as f64);
                        log::info!("Finished syncing block: {}", block_number_to_sync);
                        block_number_to_sync += 1;
                    },
                    Err(e) => {
                        log::error!("Could not get events: {:?}", e);
                        sleep(Duration::from_secs(1));
                        continue 'main;
                    },
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

fn synced_block_gauge_name(listener_id: &str) -> String {
    format!("{}_synced_block", listener_id)
}

#[cfg(test)]
pub mod tests {
    use crate::fetcher::{BlockPayInEventsFetcher, LastFinalizedBlockNumFetcher};
    use crate::listener::{Listener, PayIn};
    use crate::relay::{MockRelayer, Relay, RelayError};
    use crate::sync_checkpoint_repository::{Checkpoint, InMemoryCheckpointRepository};
    use async_trait::async_trait;
    use mockall::predicate::{always, eq};
    use mockall::*;
    use std::cmp::Ordering;
    use std::sync::Arc;
    use std::thread;
    use tokio::runtime::Handle;

    mock! {
        Fetcher {}
        #[async_trait]
        impl LastFinalizedBlockNumFetcher for Fetcher {
            async fn get_last_finalized_block_num(&mut self) -> Result<Option<u64>, ()>;
        }
        #[async_trait]
        impl BlockPayInEventsFetcher<u64, String> for Fetcher {
            async fn get_block_pay_in_events(&mut self, block_num: u64) -> Result<Vec<PayIn<u64, String>>, ()>;
        }
    }

    #[derive(Clone, Debug)]
    struct SimpleCheckpoint {
        block_num: u64,
    }

    impl PartialEq<Self> for SimpleCheckpoint {
        fn eq(&self, other: &Self) -> bool {
            self.block_num == other.block_num
        }
    }

    impl PartialOrd for SimpleCheckpoint {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            if self.block_num > other.block_num {
                Some(Ordering::Greater)
            } else if self.block_num < other.block_num {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Equal)
            }
        }
    }

    impl Checkpoint for SimpleCheckpoint {
        fn just_block_num(&self) -> bool {
            true
        }

        fn get_block_num(&self) -> u64 {
            self.block_num
        }
    }

    impl From<u64> for SimpleCheckpoint {
        fn from(value: u64) -> Self {
            SimpleCheckpoint { block_num: value }
        }
    }

    #[tokio::test]
    pub async fn sync_should_start_syncing_from_last_saved_log() {
        let handle = Handle::current();
        let mut relayer = MockRelayer::new();
        relayer
            .expect_relay()
            .times(2)
            .returning(|_, _, _, _, _| Box::pin(futures::future::ready(Ok(()))));
        let relay = Relay::Single(Arc::new(Box::new(relayer)));
        let mut fetcher = MockFetcher::new();
        fetcher.expect_get_last_finalized_block_num().times(3).returning(|| Ok(Some(3)));
        fetcher
            .expect_get_block_pay_in_events()
            .with(eq(0))
            .times(0)
            .returning(|_| Ok(vec![PayIn::new(0, None, 0, 0, [0; 32], vec![])]));
        fetcher
            .expect_get_block_pay_in_events()
            .with(eq(1))
            .times(0)
            .returning(|_| Ok(vec![PayIn::new(1, None, 0, 0, [0; 32], vec![])]));
        fetcher
            .expect_get_block_pay_in_events()
            .with(eq(2))
            .times(1)
            .returning(|_| Ok(vec![PayIn::new(2, None, 0, 0, [0; 32], vec![])]));
        fetcher
            .expect_get_block_pay_in_events()
            .with(eq(3))
            .times(1)
            .returning(|_| Ok(vec![PayIn::new(3, None, 0, 0, [0; 32], vec![])]));

        let (tx, rx) = tokio::sync::oneshot::channel();

        let checkpoint_repository: InMemoryCheckpointRepository<SimpleCheckpoint> =
            InMemoryCheckpointRepository::new(Some(SimpleCheckpoint { block_num: 1 }));

        let mut listener = Listener::new("test", handle, fetcher, relay, rx, checkpoint_repository, 0, 0).unwrap();

        let handle = thread::spawn(move || {
            let result = listener.sync();
            assert!(result.is_ok());
        });

        // give a listener some time to make a couple of tries
        thread::sleep(std::time::Duration::from_secs(3));

        // stop listener
        tx.send(()).unwrap();

        handle.join().unwrap();
    }

    #[tokio::test]
    pub async fn sync_should_keep_on_syncing_in_case_of_already_relayed_error() {
        let handle = Handle::current();
        let mut relayer = MockRelayer::new();
        relayer
            .expect_relay()
            .times(2)
            .returning(|_, _, _, _, _| Box::pin(futures::future::ready(Err(RelayError::AlreadyRelayed))));
        let relay = Relay::Single(Arc::new(Box::new(relayer)));
        let mut fetcher = MockFetcher::new();
        fetcher.expect_get_last_finalized_block_num().times(3).returning(|| Ok(Some(3)));
        fetcher
            .expect_get_block_pay_in_events()
            .with(eq(0))
            .times(0)
            .returning(|_| Ok(vec![PayIn::new(0, None, 0, 0, [0; 32], vec![])]));
        fetcher
            .expect_get_block_pay_in_events()
            .with(eq(1))
            .times(0)
            .returning(|_| Ok(vec![PayIn::new(1, None, 0, 0, [0; 32], vec![])]));
        fetcher
            .expect_get_block_pay_in_events()
            .with(eq(2))
            .times(1)
            .returning(|_| Ok(vec![PayIn::new(2, None, 0, 0, [0; 32], vec![])]));
        fetcher
            .expect_get_block_pay_in_events()
            .with(eq(3))
            .times(1)
            .returning(|_| Ok(vec![PayIn::new(3, None, 0, 0, [0; 32], vec![])]));

        let (tx, rx) = tokio::sync::oneshot::channel();

        let checkpoint_repository: InMemoryCheckpointRepository<SimpleCheckpoint> =
            InMemoryCheckpointRepository::new(Some(SimpleCheckpoint { block_num: 1 }));

        let mut listener = Listener::new("test", handle, fetcher, relay, rx, checkpoint_repository, 0, 0).unwrap();

        let handle = thread::spawn(move || {
            let result = listener.sync();
            assert!(result.is_ok());
        });

        // give a listener some time to make a couple of tries
        thread::sleep(std::time::Duration::from_secs(3));

        // stop listener
        tx.send(()).unwrap();

        handle.join().unwrap();
    }

    #[tokio::test]
    pub async fn sync_should_stop_in_case_of_relaying_other_error() {
        let handle = Handle::current();

        let mut relayer = MockRelayer::new();
        relayer
            .expect_relay()
            .times(1)
            .returning(|_, _, _, _, _| Box::pin(futures::future::ready(Err(RelayError::Other))));
        let relay = Relay::Single(Arc::new(Box::new(relayer)));

        let mut fetcher = MockFetcher::new();
        fetcher.expect_get_last_finalized_block_num().times(1).returning(|| Ok(Some(3)));
        fetcher
            .expect_get_block_pay_in_events()
            .with(eq(0))
            .times(1)
            .returning(|_| Ok(vec![PayIn::new(0, None, 0, 0, [0; 32], vec![])]));

        let (_, rx) = tokio::sync::oneshot::channel();

        let checkpoint_repository: InMemoryCheckpointRepository<SimpleCheckpoint> =
            InMemoryCheckpointRepository::new(None);

        let mut listener = Listener::new("test", handle, fetcher, relay, rx, checkpoint_repository, 0, 0).unwrap();

        let handle = thread::spawn(move || {
            let result = listener.sync();
            assert!(result.is_err());
        });

        // give a listener some time to make a couple of tries
        thread::sleep(std::time::Duration::from_secs(3));

        handle.join().unwrap();
    }

    // we should have another version of this test case where after few retries relayers sucessfully relays and listener process events from next block
    #[tokio::test]
    pub async fn sync_should_retry_relaying_in_case_of_relaying_transport_error() {
        let handle = Handle::current();

        let mut relayer = MockRelayer::new();

        relayer
            .expect_relay()
            .with(always(), eq(0), always(), always(), always())
            .times(1)
            .returning(|_, _, _, _, _| Box::pin(futures::future::ready(Ok(()))));

        relayer
            .expect_relay()
            .with(always(), eq(1), always(), always(), always())
            .times(10)
            .returning(|_, _, _, _, _| Box::pin(futures::future::ready(Err(RelayError::TransportError))));

        let relay = Relay::Single(Arc::new(Box::new(relayer)));

        let mut fetcher = MockFetcher::new();
        fetcher.expect_get_last_finalized_block_num().times(1).returning(|| Ok(Some(3)));
        fetcher.expect_get_block_pay_in_events().with(eq(0)).times(1).returning(|_| {
            Ok(vec![PayIn::new(0, None, 0, 0, [0; 32], vec![]), PayIn::new(1, None, 0, 1, [0; 32], vec![])])
        });

        let (tx, rx) = tokio::sync::oneshot::channel();

        let checkpoint_repository: InMemoryCheckpointRepository<SimpleCheckpoint> =
            InMemoryCheckpointRepository::new(None);

        let mut listener = Listener::new("test", handle, fetcher, relay, rx, checkpoint_repository, 0, 0).unwrap();

        let handle = thread::spawn(move || {
            let result = listener.sync();
            // it will error because of retry attempts exceed
            assert!(result.is_err());
        });

        // give a listener some time to make a couple of tries
        thread::sleep(std::time::Duration::from_secs(3));

        // stop listener
        tx.send(()).unwrap();

        handle.join().unwrap();
    }

    #[tokio::test]
    pub async fn sync_should_retry_relaying_in_case_of_relaying_watch_error() {
        let handle = Handle::current();

        let mut relayer = MockRelayer::new();

        relayer
            .expect_relay()
            .with(always(), eq(0), always(), always(), always())
            .times(1)
            .returning(|_, _, _, _, _| Box::pin(futures::future::ready(Ok(()))));

        relayer
            .expect_relay()
            .with(always(), eq(1), always(), always(), always())
            .times(10)
            .returning(|_, _, _, _, _| Box::pin(futures::future::ready(Err(RelayError::WatchError))));

        let relay = Relay::Single(Arc::new(Box::new(relayer)));

        let mut fetcher = MockFetcher::new();
        fetcher.expect_get_last_finalized_block_num().times(1).returning(|| Ok(Some(3)));
        fetcher.expect_get_block_pay_in_events().with(eq(0)).times(1).returning(|_| {
            Ok(vec![PayIn::new(0, None, 0, 0, [0; 32], vec![]), PayIn::new(1, None, 0, 1, [0; 32], vec![])])
        });

        let (_, rx) = tokio::sync::oneshot::channel();

        let checkpoint_repository: InMemoryCheckpointRepository<SimpleCheckpoint> =
            InMemoryCheckpointRepository::new(None);

        let mut listener = Listener::new("test", handle, fetcher, relay, rx, checkpoint_repository, 0, 0).unwrap();

        let handle = thread::spawn(move || {
            let result = listener.sync();
            // it will error because of retry attempts exceed
            assert!(result.is_err());
        });

        handle.join().unwrap();
    }
}
