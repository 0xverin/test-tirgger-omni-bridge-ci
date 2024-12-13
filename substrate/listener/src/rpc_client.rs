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

use crate::primitives::EventId;
use async_trait::async_trait;
use std::marker::PhantomData;
use subxt::backend::legacy::LegacyRpcMethods;
use subxt::backend::BlockRef;
use subxt::config::Header;
use subxt::events::{EventsClient, StaticEvent};
use subxt::{Config, OnlineClient};

pub struct BlockEvent<T> {
    pub id: EventId,
    pub event: T,
}

impl<T> BlockEvent<T> {
    pub fn new(id: EventId, event: T) -> Self {
        Self { id, event }
    }
}

pub struct PayInEvent {}

/// For fetching data from Substrate RPC node
#[async_trait]
pub trait SubstrateRpcClient {
    async fn get_last_finalized_block_num(&mut self) -> Result<u64, ()>;
    async fn get_block_pay_in_events(
        &mut self,
        block_num: u64,
    ) -> Result<Vec<BlockEvent<PayInEvent>>, ()>;
}

pub struct RpcClient<ChainConfig: Config, PayInEventType: StaticEvent> {
    legacy: LegacyRpcMethods<ChainConfig>,
    events: EventsClient<ChainConfig, OnlineClient<ChainConfig>>,
    _phantom: PhantomData<PayInEventType>,
}

impl<ChainConfig: Config, PayInEventType: StaticEvent> RpcClient<ChainConfig, PayInEventType> {}

#[async_trait]
impl<ChainConfig: Config, PayInEventType: StaticEvent + Sync + Send> SubstrateRpcClient
    for RpcClient<ChainConfig, PayInEventType>
{
    async fn get_last_finalized_block_num(&mut self) -> Result<u64, ()> {
        let finalized_header = self
            .legacy
            .chain_get_finalized_head()
            .await
            .map_err(|_| ())?;
        match self
            .legacy
            .chain_get_header(Some(finalized_header))
            .await
            .map_err(|_| ())?
        {
            Some(header) => Ok(header.number().into()),
            None => Err(()),
        }
    }
    async fn get_block_pay_in_events(
        &mut self,
        block_num: u64,
    ) -> Result<Vec<PayInEventType>, ()> {
        match self
            .legacy
            .chain_get_block_hash(Some(block_num.into()))
            .await
            .map_err(|_| ())?
        {
            Some(hash) => {
                let events = self
                    .events
                    .at(BlockRef::from_hash(hash))
                    .await
                    .map_err(|_| ())?;

                let pay_in_events = events.find::<PayInEventType>();

                Ok(pay_in_events)
            }
            None => Err(()),
        }
    }
}

#[derive(Default)]
pub struct MockedRpcClientBuilder {
    block_num: Option<u64>,
}

impl MockedRpcClientBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_block_num(mut self, block_num: u64) -> Self {
        self.block_num = Some(block_num);
        self
    }

    pub fn build(self) -> MockedRpcClient {
        MockedRpcClient {
            block_num: self.block_num.unwrap_or_default(),
        }
    }
}

pub struct MockedRpcClient {
    block_num: u64,
}

#[async_trait]
impl SubstrateRpcClient for MockedRpcClient {
    async fn get_last_finalized_block_num(&mut self) -> Result<u64, ()> {
        Ok(self.block_num)
    }

    async fn get_block_pay_in_events(
        &mut self,
        block_num: u64,
    ) -> Result<Vec<BlockEvent<PayInEvent>>, ()> {
        Ok(vec![])
    }
}

#[async_trait]
pub trait SubstrateRpcClientFactory<RpcClient: SubstrateRpcClient> {
    async fn new_client(&self) -> Result<RpcClient, ()>;
}

pub struct RpcClientFactory<ChainConfig: Config, PayInEventType: StaticEvent> {
    url: String,
    _phantom: PhantomData<(ChainConfig, PayInEventType)>,
}

impl<ChainConfig: Config, PayInEventType: StaticEvent>
    RpcClientFactory<ChainConfig, PayInEventType>
{
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<ChainConfig: Config, PayInEventType: StaticEvent + Sync + Send>
    SubstrateRpcClientFactory<RpcClient<ChainConfig, PayInEventType>>
    for RpcClientFactory<ChainConfig, PayInEventType>
{
    async fn new_client(&self) -> Result<RpcClient<ChainConfig, PayInEventType>, ()> {
        let rpc_client = subxt::backend::rpc::RpcClient::from_insecure_url(self.url.clone())
            .await
            .map_err(|e| {
                log::error!("Could not create RpcClient: {:?}", e);
            })?;
        let legacy = LegacyRpcMethods::new(rpc_client);

        let online_client = OnlineClient::from_insecure_url(self.url.clone())
            .await
            .map_err(|e| {
                log::error!("Could not create OnlineClient: {:?}", e);
            })?;
        let events = online_client.events();

        Ok(RpcClient {
            legacy,
            events,
            _phantom: PhantomData,
        })
    }
}
