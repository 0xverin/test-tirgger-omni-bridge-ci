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

use alloy::primitives::{bytes::buf::Chain, Address, Bytes, FixedBytes, Uint, U256, U8, U64, keccak256};
use subxt::utils::AccountId32;

#[derive(Debug, Clone)]
pub struct Deposit {
    pub token_address: Address,                
    pub destination_chain_id: u8,             
    pub resource_id: FixedBytes<32>,               
    pub destination_recipient_address: Bytes,
    pub depositer: Address,                   
    pub amount: U256,
    pub nonce: u64                         
}


#[derive(Debug, Clone)]
pub struct TransferFungible {
    pub bridge_chain_id: u8, 
    pub deposit_nonce: u64, 
    pub resource_id: [u8; 32], 
    pub amount: u128, 
    pub destination_recipient_address: Vec<u8> 
} 

impl TransferFungible {
    pub fn new(bridge_chain_id: u8, deposit_nonce: u64, resource_id: [u8; 32], amount: u128, destination_recipient_address: Vec<u8>) -> Self {
        Self {
            bridge_chain_id,
            deposit_nonce, 
            resource_id, 
            amount, 
            destination_recipient_address
        }
    }
    // function voteProposal(uint8 chainID, uint64 depositNonce, bytes32 resourceID, bytes32 dataHash) external onlyRelayers whenNotPaused {
    pub fn create_vote_proposal_args(self) -> (U8, U64, FixedBytes<32>, U256, Address){
        // Create proposal vote arguments here 
        let resource_id: FixedBytes<32> = FixedBytes::from(self.resource_id);
        (U8::from(self.bridge_chain_id), U64::from(self.deposit_nonce), resource_id, U256::from(self.amount), Address::from_slice(&self.destination_recipient_address))
    }

    pub fn create_call_data_and_hash(amount: U256, recipient: Address) -> (Bytes, FixedBytes<32>) {
        let mut serialized = Vec::new();
    
        // Serialize `amount` (U256) into a 32-byte array
        let mut amount_bytes = [0u8; 32];
        amount_bytes.copy_from_slice(&amount.to_be_bytes::<32>()); // Copy the amount into the padded array
        serialized.extend_from_slice(&amount_bytes); // Append the serialized amount
    
    
        // Serialize `recipient` (Address) as a 32-byte padded array
        let mut recipient_bytes_padded = [0u8; 32];
        recipient_bytes_padded[12..].copy_from_slice(recipient.as_slice()); // Right-align the 20-byte Address
        serialized.extend_from_slice(&recipient_bytes_padded); // Append the padded recipient bytes
    
        // Convert the serialized byte array into Bytes
        let serialized_bytes = Bytes::from(serialized);
    
        // Compute Keccak hash of the serialized byte array using alloy_primitives
        let hash = keccak256(&serialized_bytes);
    
        // Return both the serialized Bytes and the hash as FixedBytes<32>
        (serialized_bytes, hash)
    }
}



#[derive(Debug, Clone)]
pub enum ChainEvents{
    EthereumDepositEvent(Deposit),
    SubstrateWithdrawEvent(TransferFungible)
}

impl ChainEvents {
    pub fn construct_ethereum_event(
        token_address: Address,
        destination_chain_id: u8, 
        resource_id: FixedBytes<32>,
        destination_recipient_address: Bytes,
        depositer: Address,
        amount: U256,
        nonce: u64
    ) -> Self {
        ChainEvents::EthereumDepositEvent(
            Deposit {
                token_address, 
                destination_chain_id, 
                resource_id, 
                destination_recipient_address,
                depositer, 
                amount, 
                nonce 
            }

        )
    }

    pub fn nonce(&self) -> u64 {
        match self {
            ChainEvents::EthereumDepositEvent(deposit) => deposit.nonce, 
            ChainEvents::SubstrateWithdrawEvent(withdraw) => withdraw.deposit_nonce
        }
    }

    pub fn get_bridge_transfer_arguments(self) -> Option<(u128, [u8;32], AccountId32, u64)> {
        match self {
            ChainEvents::EthereumDepositEvent(deposit) => Some(deposit.create_bridge_transfer_arguments()), 
            ChainEvents::SubstrateWithdrawEvent(withdraw) => None
        }
    }
}

impl Deposit {
    // Consume the event 
    pub fn create_bridge_transfer_arguments(self) -> (u128, [u8;32], AccountId32, u64) {
        let amount: u128 = self.amount.try_into().unwrap();
        let resource_id: [u8;32] = self.resource_id.clone().into();
        let array: [u8; 32] = self.destination_recipient_address
            .as_ref()
            .try_into().unwrap();
        let account: AccountId32 = AccountId32(array);

        (amount, resource_id, account, self.nonce)
    }
}