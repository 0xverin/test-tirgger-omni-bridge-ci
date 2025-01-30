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
use alloy::primitives::{Address, IntoLogData};
use async_trait::async_trait;
use log::error;

use crate::primitives::{Log, LogId};
use alloy::providers::{Provider, ProviderBuilder, ReqwestProvider};
use alloy::rpc::types::Filter;

#[cfg(test)]
use mockall::automock;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ERC20Handler,
    "../chainbridge-contracts/out/ERC20Handler.sol/ERC20Handler.json"
);

use alloy::sol;

/// For fetching data from Ethereum RPC node
#[async_trait]
#[cfg_attr(test, automock)]
pub trait EthereumRpcClient {
    async fn get_block_number(&self) -> Result<u64, ()>;
    async fn get_block_logs(&self, block_number: u64, addresses: Vec<Address>, event: &str) -> Result<Vec<Log>, ()>;
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
    async fn get_block_logs(&self, block_number: u64, addresses: Vec<Address>, event: &str) -> Result<Vec<Log>, ()> {
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
}
