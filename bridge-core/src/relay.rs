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

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::mpsc;

/// Represents relayers assigned to `Listener` instance. For example PayIns from different smart contracts deployed on same EVM
/// network may be relayed to different destination chains. Strictly speaking there is a correlation between event emitter and relayer.
pub enum Relay<Id> {
    Single(Box<dyn Relayer>),
    Multi(HashMap<Id, Box<dyn Relayer>>),
}

/// Used to relay bridging request to destination chain
#[async_trait]
pub trait Relayer: Send {
    async fn relay(
        &self,
        amount: u128,
        nonce: u64,
        resource_id: [u8; 32],
        data: Vec<u8>,
    ) -> Result<(), ()>;
}

#[allow(dead_code)]
pub struct MockRelayer {
    sender: mpsc::UnboundedSender<()>,
}

impl MockRelayer {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<()>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }
}

#[async_trait]
impl Relayer for MockRelayer {
    async fn relay(
        &self,
        _amount: u128,
        _nonce: u64,
        _resource_id: [u8; 32],
        _data: Vec<u8>,
    ) -> Result<(), ()> {
        self.sender.send(()).map_err(|_| ())
    }
}
