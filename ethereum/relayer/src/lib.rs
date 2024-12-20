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

use std::thread::sleep;
use std::time::Duration;

use crate::key_store::EthereumKeyStore;
use crate::Bridge::BridgeInstance;
use alloy::hex::decode;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::{Address, U256};
use alloy::providers::fillers::{
    ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::{Identity, ProviderBuilder, RootProvider, WalletProvider};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use alloy::transports::http::{Client, Http};
use async_trait::async_trait;
use bridge_core::key_store::KeyStore;
use bridge_core::listener::DepositRecord;
use bridge_core::relay::Relayer;
use log::error;
use bridge_core::primitives::{ChainEvents, TransferFungible};

pub mod key_store;

// TODO: Update this bridge instance
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Bridge,
    "../../ethereum/chainbridge-contracts/out/Bridge.sol/Bridge.json"
);

/// Relays bridge request to smart contracts deployed on ethereum based network.
pub struct EthereumRelayer {
    bridge_instance: BridgeInstance<
        Http<Client>,
        FillProvider<
            JoinFill<
                JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>,
                WalletFiller<EthereumWallet>,
            >,
            RootProvider<Http<Client>>,
            Http<Client>,
            Ethereum,
        >,
    >,
}

// TODO: We need to configure gas options 
impl EthereumRelayer {
    pub fn new(
        rpc_url: &str,
        bridge_address: &str,
        key_store: EthereumKeyStore,
    ) -> Result<Self, ()> {
        let signer = PrivateKeySigner::from(
            key_store
                .read()
                .map_err(|e| error!("Can't read key store: {:?}", e))?,
        );

        log::debug!("The address of the local signer: {:?}", signer.address());

        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(
                rpc_url
                    .parse()
                    .map_err(|e| error!("Could not parse rpc url"))?,
            );

        let bridge_instance = Bridge::new(
            Address::from_slice(
                &decode(bridge_address).map_err(|e| error!("Can't decode bridge address"))?,
            ),
            provider,
        );

        Ok(Self { bridge_instance })
    }

    pub fn get_address(&self) -> Address {
        self.bridge_instance
            .provider()
            .signer_addresses()
            .next()
            .unwrap()
    }
}


#[async_trait]
impl Relayer for EthereumRelayer {
    async fn relay(&self, data: ChainEvents) -> Result<(), ()> {
        if let ChainEvents::SubstrateWithdrawEvent(transfer_fungible) = data {
            let transfer = transfer_fungible.clone();
            let (destination_chain_id, nonce, resource_id, amount, recipient) = transfer_fungible.create_vote_proposal_args();
            let (proposal_call_data, proposal_hash) = TransferFungible::create_call_data_and_hash(amount, recipient);

            let proposal_builder = self.bridge_instance.voteProposal(
                transfer.bridge_chain_id, 
                transfer.deposit_nonce, 
                resource_id.into(), 
                proposal_hash.into()
            ); 

            proposal_builder
                .send()
                .await
                .map_err(|e| {
                    error!("Error while sending tx: {:?}", e);
                })?
                .watch()
                .await
                .map_err(|e| {
                    error!("Error while watching tx: {:?}", e);
                })?;
            
            log::info!("Succesfully submitted voteProposal for resource_id: {:?}, amount: {:?}, recipient: {:?}", resource_id, amount, recipient);
        
            // We should also execute the proposal 
            let proposal_executer = self.bridge_instance.executeProposal(
                transfer.bridge_chain_id, 
                transfer.deposit_nonce, 
                proposal_call_data.into(), 
                resource_id.into()
            );

            log::info!("Waiting for the proposal to pass...");
            // Sleeping before the proposal passes
            sleep(Duration::from_secs(2));

            proposal_executer
                .send()
                .await
                .map_err(|e| {
                    error!("Error while sending tx: {:?}", e);
                })?
                .watch()
                .await
                .map_err(|e| {
                    error!("Error while watching tx: {:?}", e);
                })?;
            
            log::info!("Succesfully executed Proposal for resource_id: {:?}, amount: {:?}, recipient: {:?}", resource_id, amount, recipient);


        }
        Ok(())
    }
}
