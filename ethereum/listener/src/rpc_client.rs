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

use alloy::network::Ethereum;
use alloy::primitives::{Address, IntoLogData, address};
use alloy::sol_types::SolEvent;
use async_trait::async_trait;
use log::error;
use bridge_core::primitives::Deposit;

use crate::primitives::{Log, LogId};
use alloy::providers::{Provider, ProviderBuilder, ReqwestProvider};
use alloy::rpc::types::Filter;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ERC20Handler,
    "../chainbridge-contracts/out/ERC20Handler.sol/ERC20Handler.json"
);

use alloy::sol;

/// For fetching data from Ethereum RPC node
#[async_trait]
pub trait EthereumRpcClient {
    async fn get_block_number(&self) -> Result<u64, ()>;
    async fn get_block_logs(
        &self,
        block_number: u64,
        addresses: Vec<Address>,
        event: &str,
    ) -> Result<Vec<Log>, ()>;
    async fn get_deposit_record(
        &self, 
        destination_id: u8,
        deposit_nonce: u64
    ) -> Deposit;
}

pub struct EthersRpcClient {
    client: ReqwestProvider<Ethereum>,
}

impl EthersRpcClient {
    pub fn new(endpoint: &str) -> Result<Self, ()> {
        let url = endpoint.parse().map_err(|_| ())?;
        let provider = ProviderBuilder::new().on_http(url);

        Ok(EthersRpcClient { client: provider })
    }
}

#[async_trait]
impl EthereumRpcClient for EthersRpcClient {
    async fn get_block_number(&self) -> Result<u64, ()> {
        self.client.get_block_number().await.map_err(|e| {
            error!("Could not get last block number: {:?}", e);
        })
    }

    // TODO: Are there too many unwraps?
    async fn get_block_logs(
        &self,
        block_number: u64,
        addresses: Vec<Address>,
        event: &str,
    ) -> Result<Vec<Log>, ()> {
        let filter: Filter = Filter::new()
            .from_block(block_number)
            .to_block(block_number)
            .address(addresses)
            .event(event);
        self.client
            .get_logs(&filter)
            .await
            .map(|logs| {
                logs.iter()
                    .map(|log| Log {
                        id: LogId::new(
                            log.block_number.unwrap(),
                            log.transaction_index.unwrap(),
                            log.log_index.unwrap(),
                        ),
                        address: log.address(),
                        topics: log.topics().to_vec(),
                        data: log.data().to_log_data().data,
                    })
                    .collect()
            })
            .map_err(|_| ())
    }

    async fn get_deposit_record(
        &self, 
        destination_id: u8,
        deposit_nonce: u64
    ) -> Deposit {
       let contract = ERC20Handler::new(address!("e7f1725E7734CE288F8367e1Bb143E90bb3F0512"), self.client.clone());
       let deposit_record = contract.getDepositRecord(deposit_nonce, destination_id).call().await.unwrap()._0;
       Deposit {
            token_address: deposit_record._tokenAddress, 
            destination_chain_id: deposit_record._destinationChainID, 
            resource_id: deposit_record._resourceID,
            destination_recipient_address: deposit_record._destinationRecipientAddress, 
            depositer: deposit_record._depositer,
            amount: deposit_record._amount,
            nonce: deposit_nonce
       }
    }

}

#[cfg(test)]
pub mod mocks {
    use std::collections::HashMap;

    use alloy::primitives::Address;
    use async_trait::async_trait;

    use crate::primitives::Log;

    use super::EthereumRpcClient;

    #[derive(Default)]
    pub struct MockedRpcClientBuilder {
        block_number: Option<u64>,
        block_logs: Option<HashMap<u64, Vec<Log>>>,
    }

    impl MockedRpcClientBuilder {
        pub fn new() -> Self {
            Default::default()
        }

        pub fn with_block_number(mut self, block_number: u64) -> Self {
            self.block_number = Some(block_number);
            self
        }

        pub fn with_block_logs(mut self, block_logs: HashMap<u64, Vec<Log>>) -> Self {
            self.block_logs = Some(block_logs);
            self
        }

        pub fn build(self) -> MockedRpcClient {
            MockedRpcClient {
                block_number: self.block_number.unwrap_or_default(),
                block_logs: self.block_logs.unwrap_or_default(),
            }
        }
    }

    pub struct MockedRpcClient {
        block_number: u64,
        block_logs: HashMap<u64, Vec<Log>>,
    }

    #[async_trait]
    impl EthereumRpcClient for MockedRpcClient {
        async fn get_block_number(&self) -> Result<u64, ()> {
            Ok(self.block_number)
        }

        async fn get_block_logs(
            &self,
            block_number: u64,
            addresses: Vec<Address>,
            event: &str,
        ) -> Result<Vec<Log>, ()> {
            self.block_logs
                .get(&block_number)
                .map(|logs| logs.to_owned())
                .ok_or(())
        }
    }
}
