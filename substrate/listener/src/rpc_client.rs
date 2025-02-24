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
use crate::PalletPaidInEvent;
use async_trait::async_trait;
use std::marker::PhantomData;
use subxt::backend::legacy::LegacyRpcMethods;
use subxt::backend::BlockRef;
use subxt::config::Header;
use subxt::events::EventsClient;
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

pub struct PaidInEvent {
    pub amount: u128,
    pub nonce: u64,
    pub resource_id: [u8; 32],
    pub data: Vec<u8>,
    // scale encoded chain type from omni-bridge pallet
    pub dest_chain: Vec<u8>,
}

/// For fetching data from Substrate RPC node
#[async_trait]
pub trait SubstrateRpcClient {
    async fn get_last_finalized_block_num(&mut self) -> Result<u64, ()>;
    async fn get_block_pay_in_events(&mut self, block_num: u64) -> Result<Vec<BlockEvent<PaidInEvent>>, ()>;
}

pub struct RpcClient<ChainConfig: Config, PalletPaidInEventType: PalletPaidInEvent> {
    legacy: LegacyRpcMethods<ChainConfig>,
    events: EventsClient<ChainConfig, OnlineClient<ChainConfig>>,
    phantom_data: PhantomData<PalletPaidInEventType>,
}

impl<ChainConfig: Config, PalletPaidInEventType: PalletPaidInEvent> RpcClient<ChainConfig, PalletPaidInEventType> {}

#[async_trait]
impl<ChainConfig: Config, PalletPaidInEventType: PalletPaidInEvent> SubstrateRpcClient
    for RpcClient<ChainConfig, PalletPaidInEventType>
{
    async fn get_last_finalized_block_num(&mut self) -> Result<u64, ()> {
        let finalized_header = self.legacy.chain_get_finalized_head().await.map_err(|_| ())?;
        match self.legacy.chain_get_header(Some(finalized_header)).await.map_err(|_| ())? {
            Some(header) => Ok(header.number().into()),
            None => Err(()),
        }
    }
    async fn get_block_pay_in_events(&mut self, block_num: u64) -> Result<Vec<BlockEvent<PaidInEvent>>, ()> {
        match self.legacy.chain_get_block_hash(Some(block_num.into())).await.map_err(|_| ())? {
            Some(hash) => {
                let events = self.events.at(BlockRef::from_hash(hash)).await.map_err(|_| ())?;

                let pay_in_events = events.find::<PalletPaidInEventType::MetadataType>();

                Ok(pay_in_events
                    .enumerate()
                    .map(|(i, event)| {
                        let event: PalletPaidInEventType = PalletPaidInEventType::wrap(event.unwrap());
                        BlockEvent::new(
                            EventId::new(block_num, i as u64),
                            PaidInEvent {
                                amount: event.amount(),
                                resource_id: event.resource_id(),
                                data: event.dest_account(),
                                nonce: event.nonce(),
                                dest_chain: event.dest_chain(),
                            },
                        )
                    })
                    .collect())
            },
            None => Err(()),
        }
    }
}

#[async_trait]
pub trait SubstrateRpcClientFactory<RpcClient: SubstrateRpcClient> {
    async fn new_client(&self) -> Result<RpcClient, ()>;
}

pub struct RpcClientFactory<ChainConfig: Config> {
    url: String,
    _phantom: PhantomData<ChainConfig>,
}

impl<ChainConfig: Config> RpcClientFactory<ChainConfig> {
    pub fn new(url: &str) -> Self {
        Self { url: url.to_string(), _phantom: PhantomData }
    }
}

#[async_trait]
impl<ChainConfig: Config, PalletPaidInEventType: PalletPaidInEvent>
    SubstrateRpcClientFactory<RpcClient<ChainConfig, PalletPaidInEventType>> for RpcClientFactory<ChainConfig>
{
    async fn new_client(&self) -> Result<RpcClient<ChainConfig, PalletPaidInEventType>, ()> {
        let rpc_client = subxt::backend::rpc::RpcClient::from_insecure_url(self.url.clone())
            .await
            .map_err(|e| {
                log::error!("Could not create RpcClient: {:?}", e);
            })?;
        let legacy = LegacyRpcMethods::new(rpc_client);

        let online_client = OnlineClient::from_insecure_url(self.url.clone()).await.map_err(|e| {
            log::error!("Could not create OnlineClient: {:?}", e);
        })?;
        let events = online_client.events();

        Ok(RpcClient { legacy, events, phantom_data: PhantomData })
    }
}
