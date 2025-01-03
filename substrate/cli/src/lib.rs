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

use crate::litentry_rococo::chain_bridge::Call;
use crate::litentry_rococo::runtime_types::pallet_bridge_common::AssetInfo;
use crate::litentry_rococo::runtime_types::rococo_parachain_runtime::RuntimeCall;
use clap::Subcommand;
use hex::FromHex;
use std::str::FromStr;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::dev;

#[subxt::subxt(runtime_metadata_path = "../artifacts/metadata.scale")]
pub mod litentry_rococo {}

#[derive(Subcommand)]
pub enum SubstrateCommand {
    SetupDevChainBridge,
    Balance { account: String },
    DevTestWithdraw,
}

pub async fn handle(command: &SubstrateCommand) {
    let rpc_url = "ws://localhost:9944";
    let alice_signer = dev::alice();
    log::info!("Alice: {:?}", alice_signer);

    let api = OnlineClient::<PolkadotConfig>::from_insecure_url(rpc_url)
        .await
        .unwrap();

    match command {
        SubstrateCommand::SetupDevChainBridge => {
            let add_relayer_call = RuntimeCall::ChainBridge(Call::add_relayer {
                v: alice_signer.public_key().into(),
            });

            let add_relayer_sudo_call = litentry_rococo::tx().sudo().sudo(add_relayer_call);

            println!("Adding Relayer to the ChainBridge Pallet");
            let hash = api
                .tx()
                .sign_and_submit_then_watch(
                    &add_relayer_sudo_call,
                    &alice_signer,
                    Default::default(),
                )
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let whitelist_chain_id = RuntimeCall::ChainBridge(Call::whitelist_chain { id: 0 });

            let whitelist_chain_id_sudo_call =
                litentry_rococo::tx().sudo().sudo(whitelist_chain_id);

            println!("Whitelisting Ethereum Chain ID in ChainBridge Pallet");
            let hash = api
                .tx()
                .sign_and_submit_then_watch(
                    &whitelist_chain_id_sudo_call,
                    &alice_signer,
                    Default::default(),
                )
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let set_threshold_call = RuntimeCall::ChainBridge(Call::set_threshold { threshold: 1 });

            let set_threshold_sudo_call = litentry_rococo::tx().sudo().sudo(set_threshold_call);

            println!("Setting Relayer Threshold in ChainBridge Pallet");
            let hash = api
                .tx()
                .sign_and_submit_then_watch(
                    &set_threshold_sudo_call,
                    &alice_signer,
                    Default::default(),
                )
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let resource_id: [u8; 32] = <[u8; 32]>::from_hex(
                "6dbf3f9d61108d592cb424722ba78a9c2e786a3d1436508c9a02d7e48d70e41e",
            )
            .expect("Failed to decode hex string");
            let asset = AssetInfo {
                fee: 0,
                asset: None,
            };

            let set_resource_call =
                RuntimeCall::AssetsHandler(litentry_rococo::assets_handler::Call::set_resource {
                    resource_id,
                    asset,
                });

            let set_resource_sudo_call = litentry_rococo::tx().sudo().sudo(set_resource_call);

            println!("Setting Resource in Assets Handler");
            let hash = api
                .tx()
                .sign_and_submit_then_watch(
                    &set_resource_sudo_call,
                    &alice_signer,
                    Default::default(),
                )
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();
        }
        SubstrateCommand::Balance { account } => {
            // Query the account balance from the chain's `Balances` storage
            let account: AccountId32 = AccountId32::from_str(account).unwrap();

            let balances_storage_query = litentry_rococo::storage().system().account(account);
            let balances_details = api
                .storage()
                .at_latest()
                .await
                .unwrap()
                .fetch(&balances_storage_query)
                .await
                .unwrap()
                .ok_or("There is no account with existential deposit");

            if let Ok(details) = balances_details {
                let free_balance = details.data.free;
                println!("{:?}", free_balance)
            } else {
                println!("0");
            }
        }
        SubstrateCommand::DevTestWithdraw => {
            let recipient_address = Vec::<u8>::from_hex("0e209C5dEdFfE34120679A681a0d7d21A360a97f")
                .expect("Failed to decode string");
            let resource_id: [u8; 32] = <[u8; 32]>::from_hex(
                "6dbf3f9d61108d592cb424722ba78a9c2e786a3d1436508c9a02d7e48d70e41e",
            )
            .expect("Failed to decode hex string");
            let amount: u128 = 10_000_000_000_000_000_000;

            let transfer_assets_call = litentry_rococo::tx().bridge_transfer().transfer_assets(
                amount,
                recipient_address,
                0,
                resource_id,
            );

            let hash = api
                .tx()
                .sign_and_submit_then_watch(
                    &transfer_assets_call,
                    &alice_signer,
                    Default::default(),
                )
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();
        }
    }
}
