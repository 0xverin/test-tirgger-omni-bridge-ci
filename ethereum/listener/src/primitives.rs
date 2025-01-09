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

use alloy::primitives::{Address, Bytes, B256};
use bridge_core::sync_checkpoint_repository::Checkpoint;
use parity_scale_codec::{Decode, Encode};

/// Represents ethereum based chain sync checkpoint.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct SyncCheckpoint {
    pub block_num: u64,
    pub tx_idx: Option<u64>,
    pub log_idx: Option<u64>,
}

impl SyncCheckpoint {
    pub fn new(block_num: u64, tx_idx: Option<u64>, log_idx: Option<u64>) -> Self {
        Self { block_num, tx_idx, log_idx }
    }

    pub fn from_log_id(id: &LogId) -> Self {
        Self::new(id.block_num, Some(id.tx_idx), Some(id.log_idx))
    }

    pub fn from_block_num(block_num: u64) -> Self {
        Self::new(block_num, None, None)
    }

    pub fn just_block_num(&self) -> bool {
        self.log_idx.is_none() && self.tx_idx.is_none()
    }
}

impl Checkpoint for SyncCheckpoint {
    fn just_block_num(&self) -> bool {
        self.just_block_num()
    }

    fn get_block_num(&self) -> u64 {
        self.block_num
    }
}

impl From<u64> for SyncCheckpoint {
    fn from(value: u64) -> Self {
        Self::from_block_num(value)
    }
}

impl From<LogId> for SyncCheckpoint {
    fn from(value: LogId) -> Self {
        Self::from_log_id(&value)
    }
}

impl PartialOrd for SyncCheckpoint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.block_num > other.block_num {
            Some(std::cmp::Ordering::Greater)
        } else if self.block_num < other.block_num {
            Some(std::cmp::Ordering::Less)
        } else if self.tx_idx > other.tx_idx {
            Some(std::cmp::Ordering::Greater)
        } else if self.tx_idx < other.tx_idx {
            Some(std::cmp::Ordering::Less)
        } else if self.log_idx > other.log_idx {
            Some(std::cmp::Ordering::Greater)
        } else if self.log_idx < other.log_idx {
            Some(std::cmp::Ordering::Less)
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Log {
    pub id: LogId,
    pub address: Address,
    pub topics: Vec<B256>,
    pub data: Bytes,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogId {
    pub block_num: u64,
    pub tx_idx: u64,
    pub log_idx: u64,
}

impl LogId {
    pub fn new(block_num: u64, tx_idx: u64, log_idx: u64) -> Self {
        LogId { block_num, tx_idx, log_idx }
    }
}

#[cfg(test)]
mod tests {

    use crate::primitives::SyncCheckpoint;

    #[test]
    pub fn checkpoint_lower_block_number() {
        let id_1 = SyncCheckpoint::from_block_num(1);
        let id_2 = SyncCheckpoint::from_block_num(2);

        assert!(id_1.lt(&id_2))
    }
}
