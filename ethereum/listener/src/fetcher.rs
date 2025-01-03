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

// sepolia address
// 0xb77cbea4b8f4d176b6999d0c22a9ce8e1303483d

use crate::listener::{EventSourceId, PayInEventId};
use crate::rpc_client::EthereumRpcClient;
use alloy::primitives::{keccak256, Address, B256, U256};
use alloy::sol;
use alloy::sol_types::{SolEvent, SolValue};
use async_trait::async_trait;
use bridge_core::fetcher::{BlockPayInEventsFetcher, LastFinalizedBlockNumFetcher};
use bridge_core::listener::PayIn;
use std::collections::HashSet;

pub static EVENT_TOPIC: &str = "Deposit(uint8,bytes32,uint64,address,bytes,bytes)";

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ChainBridge,
    "../chainbridge-contracts/out/Bridge.sol/Bridge.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ERC20Handler,
    "../chainbridge-contracts/out/ERC20Handler.sol/ERC20Handler.json"
);

/// Used for fetching data from ethereum based chains required by the `Listener`
pub struct Fetcher<RpcClient> {
    finalization_gap_blocks: u64,
    client: RpcClient,
    event_sources: HashSet<Address>,
    event_topic: B256,
}

impl<C> Fetcher<C> {
    pub fn new(finalization_gap_blocks: u64, client: C, event_sources: HashSet<Address>) -> Self {
        Self {
            finalization_gap_blocks,
            client,
            event_sources,
            event_topic: keccak256(EVENT_TOPIC.as_bytes()),
        }
    }
}

#[async_trait]
impl<C: EthereumRpcClient + Sync + Send> LastFinalizedBlockNumFetcher for Fetcher<C> {
    async fn get_last_finalized_block_num(&mut self) -> Result<Option<u64>, ()> {
        let last_block_number = self.client.get_block_number().await?;
        Ok(last_block_number.checked_sub(self.finalization_gap_blocks))
    }
}

#[async_trait]
impl<C: EthereumRpcClient + Sync + Send> BlockPayInEventsFetcher<PayInEventId, EventSourceId>
    for Fetcher<C>
{
    async fn get_block_pay_in_events(
        &mut self,
        block_num: u64,
    ) -> Result<Vec<PayIn<PayInEventId, EventSourceId>>, ()> {
        let block_logs = self
            .client
            .get_block_logs(
                block_num,
                Vec::from_iter(self.event_sources.clone()),
                EVENT_TOPIC,
            )
            .await?;

        log::debug!("Checking log details for block number: {:?}", block_num);
        log::debug!(
            "Checking log details for contract: {:?}",
            self.event_sources
        );
        log::debug!("Checking log details for topic: {:?}", self.event_topic);
        log::debug!("Size of the logs received via RPC: {:?}", block_logs.len());
        log::debug!("Logs in the buffer: {:?}", block_logs);

        let deposit_events: Vec<_> = block_logs
            .into_iter()
            .filter(|log| {
                self.event_sources.contains(&log.address) && log.topics.contains(&self.event_topic)
            })
            .map(|log| {
                let event = ChainBridge::Deposit::abi_decode_data(&log.data, false).unwrap();
                log::debug!("Got contract events: {:?}", event);
                let nonce = event.2;
                let data = event.3;

                let amount_bytes = &data[0..32];
                let amount: U256 = U256::abi_decode(amount_bytes, false).unwrap();

                PayIn::new(
                    log.id,
                    Some(log.address),
                    amount.try_into().unwrap(),
                    nonce,
                    data.into(),
                )
            })
            .collect();

        log::info!("Found {:?} Deposits on Ethereum", deposit_events.len());
        Ok(deposit_events)
    }
}

#[cfg(test)]
mod test {
    use super::{Fetcher, EVENT_TOPIC};

    use crate::listener::{EthereumPayInEvent, PayInEventId};
    use crate::primitives::LogId;
    use crate::{primitives::Log, rpc_client::mocks::MockedRpcClientBuilder};
    use alloy::dyn_abi::DynSolValue;
    use alloy::primitives::{keccak256, Address, Bytes, U160, U256};
    use alloy::sol_types::SolValue;
    use bridge_core::fetcher::BlockPayInEventsFetcher;
    use bridge_core::listener::PayIn;
    use std::collections::{HashMap, HashSet};

    #[tokio::test]
    async fn it_should_return_contract_logs() {
        // given
        let source = Address::from(U160::from(150));
        let mut pay_in_events: HashMap<u64, Vec<EthereumPayInEvent>> = HashMap::new();
        let mut logs: HashMap<u64, Vec<Log>> = HashMap::new();

        let event_data = U256::from(10).abi_encode();

        let block_1_logs: Vec<Log> = vec![Log {
            id: LogId::new(1, 1, 1),
            address: source,
            topics: vec![keccak256(EVENT_TOPIC.as_bytes())],
            data: Bytes::from(
                DynSolValue::Tuple(vec![
                    DynSolValue::Uint(U256::from(0), 8),
                    DynSolValue::Uint(U256::from(0), 256),
                    DynSolValue::Uint(U256::from(1), 64),
                    DynSolValue::Bytes(event_data.to_vec()),
                    DynSolValue::Uint(U256::from(10), 256),
                ])
                .abi_encode_params(),
            ),
        }];
        let block_2_logs: Vec<Log> = vec![];

        logs.insert(1, block_1_logs);
        logs.insert(2, block_2_logs);

        let block_1_pay_in_events: Vec<EthereumPayInEvent> = vec![PayIn::new(
            PayInEventId::new(1, 1, 1),
            Some(source),
            10,
            1,
            event_data,
        )];
        let block_2_pay_in_events: Vec<EthereumPayInEvent> = vec![];

        pay_in_events.insert(1, block_1_pay_in_events.clone());
        pay_in_events.insert(2, block_2_pay_in_events.clone());

        let rpc_client = MockedRpcClientBuilder::new().with_block_logs(logs).build();
        let mut fetcher = Fetcher::new(0, rpc_client, HashSet::from_iter(vec![source]));

        // when and then -.-
        assert_eq!(
            block_1_pay_in_events,
            fetcher.get_block_pay_in_events(1).await.unwrap()
        );
        assert_eq!(
            block_2_pay_in_events,
            fetcher.get_block_pay_in_events(2).await.unwrap()
        );
    }
}
