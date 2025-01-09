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

use crate::litentry_rococo::omni_bridge::Call;
use clap::{Args, Subcommand};
use hex::FromHex;
use log::info;
use std::str::FromStr;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::dev;

#[subxt::subxt(runtime_metadata_path = "../artifacts/rococo-bridge.scale")]
pub mod litentry_rococo {}

#[derive(Subcommand)]
pub enum SubstrateCommand {
    SetupDevChainBridge(SetupDevChainBridgeCmdConf),
    Balance { account: String },
    DevTestWithdraw,
}

#[derive(Args)]
pub struct SetupDevChainBridgeCmdConf {
    #[clap(default_value = "5FkFYAaxiPnrQiMSZZPLABfiKazRGKU2rVZmki1iSVe15PXa")]
    relayer_account: String,
}

pub async fn handle(command: &SubstrateCommand) {
    let rpc_url = "ws://localhost:9944";
    let alice_signer = dev::alice();

    let api = OnlineClient::<PolkadotConfig>::from_insecure_url(rpc_url)
        .await
        .unwrap();

    match command {
        SubstrateCommand::SetupDevChainBridge(conf) => {
            let add_relayer_call = crate::litentry_rococo::runtime_types::paseo_parachain_runtime::RuntimeCall::OmniBridge(Call::add_relayer {
                who: AccountId32::from_str(&conf.relayer_account).unwrap()
            });

            let add_relayer_sudo_call = litentry_rococo::tx().sudo().sudo(add_relayer_call);

            info!("Adding Relayer to the OmniBridge Pallet");
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

            let chain_asset = litentry_rococo::runtime_types::pallet_omni_bridge::ChainAsset {
                chain: litentry_rococo::runtime_types::pallet_omni_bridge::ChainType::Heima,
                asset: litentry_rococo::runtime_types::frame_support::traits::tokens::fungible::union_of::NativeOrWithId::Native
            };

            info!("Setting ResourceId on OmniBridge Pallet");
            let set_resource_id_call = litentry_rococo::tx()
                .omni_bridge()
                .set_resource_id([158, 230, 223, 182, 26, 47, 185, 3, 223, 72, 124, 64, 22, 99, 130, 86, 67, 187, 130, 93, 65, 105, 94, 99, 223, 138, 246, 22, 42, 177, 69, 166], chain_asset);

            let hash = api
                .tx()
                .sign_and_submit_then_watch(
                    &set_resource_id_call,
                    &alice_signer,
                    Default::default(),
                )
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let asset_kind = litentry_rococo::runtime_types::frame_support::traits::tokens::fungible::union_of::NativeOrWithId::Native;
            let dest_chain =
                litentry_rococo::runtime_types::pallet_omni_bridge::ChainType::Ethereum(0);

            info!("Adding pay in pair on OmniBridgePallet");
            let add_pay_in_pair_call = litentry_rococo::tx()
                .omni_bridge()
                .add_pay_in_pair(asset_kind, dest_chain);

            let hash = api
                .tx()
                .sign_and_submit_then_watch(
                    &add_pay_in_pair_call,
                    &alice_signer,
                    Default::default(),
                )
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let asset_kind = litentry_rococo::runtime_types::frame_support::traits::tokens::fungible::union_of::NativeOrWithId::Native;
            let dest_chain =
                litentry_rococo::runtime_types::pallet_omni_bridge::ChainType::Ethereum(0);

            // set pay in fee
            info!("Setting pay in fee on OmniBridgePallet");
            let set_pay_in_fee = litentry_rococo::tx()
                .omni_bridge()
                .set_pay_in_fee(asset_kind, dest_chain, 0);
            let hash = api
                .tx()
                .sign_and_submit_then_watch(
                    &set_pay_in_fee,
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
            let recipient_address = Vec::<u8>::from_hex("70997970C51812dc3A010C7d01b50e0d17dc79C8")
                .expect("Failed to decode string");

            let amount: u128 = 10000;

            let request = litentry_rococo::runtime_types::pallet_omni_bridge::PayInRequest {
                asset: litentry_rococo::runtime_types::frame_support::traits::tokens::fungible::union_of::NativeOrWithId::Native,
                dest_chain: litentry_rococo::runtime_types::pallet_omni_bridge::ChainType::Ethereum(0),
                dest_account: recipient_address,
                amount,
            };

            let transfer_assets_call = litentry_rococo::tx().omni_bridge().pay_in(request);

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
