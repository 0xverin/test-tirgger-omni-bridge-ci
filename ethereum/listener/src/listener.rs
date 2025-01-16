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
use alloy::primitives::Address;
use bridge_core::listener::{Listener, PayIn};
use serde::Deserialize;

pub type PayInEventId = LogId;
pub type EventSourceId = Address;
pub type EthereumPayInEvent = PayIn<PayInEventId, EventSourceId>;

#[derive(Deserialize)]
pub struct ListenerConfig {
    pub node_rpc_url: String,
    pub bridge_contract_address: String,
}

pub type EthereumListener<RpcClient, CheckpointRepository> =
    Listener<EventSourceId, Fetcher<RpcClient>, SyncCheckpoint, CheckpointRepository, PayInEventId>;

#[cfg(test)]
pub mod tests {
    use std::{
        collections::HashMap,
        str::FromStr,
        thread::{self},
        time::Duration,
    };

    use bridge_core::relay::{MockRelayer, Relay, Relayer};
    use bridge_core::sync_checkpoint_repository::InMemoryCheckpointRepository;

    use alloy::dyn_abi::DynSolValue;
    use alloy::primitives::{keccak256, Address, Bytes, U256};
    use alloy::sol_types::SolValue;
    use thread::sleep;
    use tokio::{
        runtime::Handle,
        sync::{mpsc::UnboundedReceiver, oneshot},
    };

    use crate::fetcher::{Fetcher, EVENT_TOPIC};
    use crate::{listener::EthereumListener, primitives::SyncCheckpoint, rpc_client::mocks::MockedRpcClient};
    use crate::{
        primitives::{Log, LogId},
        rpc_client::mocks::MockedRpcClientBuilder,
    };

    #[tokio::test]
    pub async fn it_should_relay() {
        let _ = env_logger::builder().is_test(true).try_init();

        // given
        let source = "0x0000000000000000000000000000000000000001";
        let (relay, mut receiver) = MockRelayer::new();
        let start_block = 0;
        let mut logs: HashMap<u64, Vec<Log>> = HashMap::new();

        let event_data = U256::from(10).abi_encode();

        let block_1_logs: Vec<Log> = vec![Log {
            id: LogId::new(1, 1, 1),
            address: Address::from_str(source).unwrap(),
            topics: vec![keccak256(EVENT_TOPIC.as_bytes())],
            data: Bytes::from(
                DynSolValue::Tuple(vec![
                    DynSolValue::Uint(U256::from(0), 8),
                    DynSolValue::Uint(U256::from(0), 256),
                    DynSolValue::Address(Address::default()),
                    DynSolValue::Bytes(event_data.to_vec()),
                    DynSolValue::Uint(U256::from(10), 256),
                ])
                .abi_encode_params(),
            ),
        }];
        logs.insert(0, vec![]);

        logs.insert(1, block_1_logs.clone());

        let rpc_client = MockedRpcClientBuilder::new().with_block_logs(logs).with_block_number(1).build();

        let mut relay_map: HashMap<Address, Box<dyn Relayer>> = HashMap::new();
        relay_map.insert(Address::from_str(source).map_err(|_| ()).unwrap(), Box::new(relay));

        let fetcher: Fetcher<MockedRpcClient> = Fetcher::new(0, rpc_client, relay_map.keys().copied().collect());

        let (stop_sender, stop_receiver) = oneshot::channel();

        let mut listener: EthereumListener<MockedRpcClient, InMemoryCheckpointRepository<SyncCheckpoint>> =
            EthereumListener::new(
                "test",
                Handle::current().clone(),
                fetcher,
                Relay::Multi(relay_map),
                stop_receiver,
                InMemoryCheckpointRepository::new(None),
                start_block,
            )
            .unwrap();

        let _handle = thread::spawn(move || listener.sync());

        assert_relay_count(&mut receiver, 1).await;
        stop_sender.send(()).unwrap();
    }

    #[tokio::test]
    pub async fn it_should_start_relying_from_last_saved_log() {
        let _ = env_logger::builder().is_test(true).try_init();

        // given
        let source = "0x0000000000000000000000000000000000000001";
        let (relay, mut receiver) = MockRelayer::new();
        let start_block = 0;
        let mut logs: HashMap<u64, Vec<Log>> = HashMap::new();

        let event_data = U256::from(10).abi_encode();

        let block_1_logs: Vec<Log> = vec![Log {
            id: LogId::new(1, 1, 1),
            address: Address::from_str(source).unwrap(),
            topics: vec![keccak256(EVENT_TOPIC.as_bytes())],
            data: Bytes::from(
                DynSolValue::Tuple(vec![
                    DynSolValue::Uint(U256::from(0), 8),
                    DynSolValue::Uint(U256::from(0), 256),
                    DynSolValue::Address(Address::default()),
                    DynSolValue::Bytes(event_data.to_vec()),
                    DynSolValue::Uint(U256::from(10), 256),
                ])
                .abi_encode_params(),
            ),
        }];

        let block_2_logs: Vec<Log> = vec![
            Log {
                id: LogId::new(2, 1, 1),
                address: Address::from_str(source).unwrap(),
                topics: vec![keccak256(EVENT_TOPIC.as_bytes())],
                data: Bytes::from(
                    DynSolValue::Tuple(vec![
                        DynSolValue::Uint(U256::from(0), 8),
                        DynSolValue::Uint(U256::from(0), 256),
                        DynSolValue::Address(Address::default()),
                        DynSolValue::Bytes(event_data.to_vec()),
                        DynSolValue::Uint(U256::from(10), 256),
                    ])
                    .abi_encode_params(),
                ),
            },
            Log {
                id: LogId::new(2, 1, 2),
                address: Address::from_str(source).unwrap(),
                topics: vec![keccak256(EVENT_TOPIC.as_bytes())],
                data: Bytes::from(
                    DynSolValue::Tuple(vec![
                        DynSolValue::Uint(U256::from(0), 8),
                        DynSolValue::Uint(U256::from(0), 256),
                        DynSolValue::Address(Address::default()),
                        DynSolValue::Bytes(event_data.to_vec()),
                        DynSolValue::Uint(U256::from(10), 256),
                    ])
                    .abi_encode_params(),
                ),
            },
        ];

        logs.insert(0, vec![]);

        logs.insert(1, block_1_logs.clone());

        logs.insert(2, block_2_logs.clone());

        let rpc_client = MockedRpcClientBuilder::new().with_block_logs(logs).with_block_number(2).build();
        let (stop_sender, stop_receiver) = oneshot::channel();

        let mut relay_map: HashMap<Address, Box<dyn Relayer>> = HashMap::new();
        relay_map.insert(Address::from_str(source).map_err(|_| ()).unwrap(), Box::new(relay));

        let fetcher: Fetcher<MockedRpcClient> = Fetcher::new(0, rpc_client, relay_map.keys().copied().collect());

        let mut listener: EthereumListener<MockedRpcClient, InMemoryCheckpointRepository<SyncCheckpoint>> =
            EthereumListener::new(
                "test",
                Handle::current().clone(),
                fetcher,
                Relay::Multi(relay_map),
                stop_receiver,
                InMemoryCheckpointRepository::new(Some(SyncCheckpoint::new(2, Some(1), Some(2)))),
                start_block,
            )
            .unwrap();

        let _handle = thread::spawn(move || listener.sync());

        assert_relay_count(&mut receiver, 1).await;

        stop_sender.send(()).unwrap();
    }

    async fn assert_relay_count(receiver: &mut UnboundedReceiver<()>, count: u64) {
        // let's give some time listener to process blocks
        sleep(Duration::from_millis(100));
        for _i in 0..count {
            log::info!("Waiting for event");
            assert!(receiver.recv().await.is_some());
            log::info!("Got event");
        }
        assert!(receiver.try_recv().is_err(), "Received more events than expected");
    }
}
