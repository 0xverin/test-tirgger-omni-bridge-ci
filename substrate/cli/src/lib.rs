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
use crate::litentry_rococo::system::events::ExtrinsicFailed;
use crate::litentry_rococo::DispatchError;
use clap::{Args, Subcommand};
use hex::FromHex;
use log::info;
use std::str::FromStr;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::dev;

#[subxt::subxt(runtime_metadata_path = "../artifacts/local.scale")]
pub mod litentry_rococo {}

#[derive(Subcommand)]
pub enum SubstrateCommand {
    SetupBridge(SetupBridgeConf),
    PayIn(PayInConf),
    Balance(BalanceConf),
    FailedBridgeTx,
}

#[derive(Args)]
pub struct SetupBridgeConf {
    #[arg(long, default_value = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY")]
    relayer_account: String,
}

#[derive(Args)]
pub struct PayInConf {
    #[arg(long, default_value = "70997970C51812dc3A010C7d01b50e0d17dc79C8")]
    dest_address: String,
    #[arg(long, default_value = "100000000000000000000")] // 100 LIT
    amount: u128,
    #[arg(long, default_value = "0")] // ethereum main network
    ethereum_id: u32,
}

#[derive(Args)]
pub struct BalanceConf {
    #[arg(long)]
    account: String,
}

pub async fn handle(command: &SubstrateCommand) {
    let rpc_url = "ws://localhost:9944";
    let alice_signer = dev::alice();

    let api = OnlineClient::<PolkadotConfig>::from_insecure_url(rpc_url).await.unwrap();

    match command {
        SubstrateCommand::SetupBridge(conf) => {
            if conf.relayer_account.as_str() != "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" {
                let add_relayer_call =
                    crate::litentry_rococo::runtime_types::paseo_runtime::RuntimeCall::OmniBridge(Call::add_relayer {
                        who: AccountId32::from_str(&conf.relayer_account).unwrap(),
                    });

                let add_relayer_sudo_call = litentry_rococo::tx().sudo().sudo(add_relayer_call);

                info!("Adding Relayer to the OmniBridge Pallet");
                let hash = api
                    .tx()
                    .sign_and_submit_then_watch(&add_relayer_sudo_call, &alice_signer, Default::default())
                    .await
                    .unwrap();

                hash.wait_for_finalized().await.unwrap();
            }

            let chain_asset = litentry_rococo::runtime_types::pallet_omni_bridge::ChainAsset {
                chain: crate::litentry_rococo::runtime_types::core_primitives::omni::chain::ChainType::Heima,
                asset: litentry_rococo::runtime_types::frame_support::traits::tokens::fungible::union_of::NativeOrWithId::Native
            };

            info!("Setting ResourceId on OmniBridge Pallet");
            let set_resource_id_call = litentry_rococo::tx().omni_bridge().set_resource_id(
                [
                    158, 230, 223, 182, 26, 47, 185, 3, 223, 72, 124, 64, 22, 99, 130, 86, 67, 187, 130, 93, 65, 105,
                    94, 99, 223, 138, 246, 22, 42, 177, 69, 166,
                ],
                chain_asset,
            );

            let hash = api
                .tx()
                .sign_and_submit_then_watch(&set_resource_id_call, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let asset_kind = litentry_rococo::runtime_types::frame_support::traits::tokens::fungible::union_of::NativeOrWithId::Native;
            let dest_chain =
                crate::litentry_rococo::runtime_types::core_primitives::omni::chain::ChainType::Ethereum(0);

            info!("Adding pay in pair on OmniBridgePallet");
            let add_pay_in_pair_call = litentry_rococo::tx().omni_bridge().add_pay_in_pair(asset_kind, dest_chain);

            let hash = api
                .tx()
                .sign_and_submit_then_watch(&add_pay_in_pair_call, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let asset_kind = litentry_rococo::runtime_types::frame_support::traits::tokens::fungible::union_of::NativeOrWithId::Native;
            let dest_chain =
                crate::litentry_rococo::runtime_types::core_primitives::omni::chain::ChainType::Ethereum(56);

            info!("Adding pay in pair on OmniBridgePallet");
            let add_pay_in_pair_call = litentry_rococo::tx().omni_bridge().add_pay_in_pair(asset_kind, dest_chain);

            let hash = api
                .tx()
                .sign_and_submit_then_watch(&add_pay_in_pair_call, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let asset_kind = litentry_rococo::runtime_types::frame_support::traits::tokens::fungible::union_of::NativeOrWithId::Native;
            let dest_chain =
                crate::litentry_rococo::runtime_types::core_primitives::omni::chain::ChainType::Ethereum(0);

            // set pay in fee
            info!("Setting pay in fee on OmniBridgePallet");
            let set_pay_in_fee = litentry_rococo::tx().omni_bridge().set_pay_in_fee(asset_kind, dest_chain, 0);
            let hash = api
                .tx()
                .sign_and_submit_then_watch(&set_pay_in_fee, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let asset_kind = litentry_rococo::runtime_types::frame_support::traits::tokens::fungible::union_of::NativeOrWithId::Native;
            let dest_chain =
                crate::litentry_rococo::runtime_types::core_primitives::omni::chain::ChainType::Ethereum(56);

            // set pay in fee
            info!("Setting pay in fee on OmniBridgePallet");
            let set_pay_in_fee = litentry_rococo::tx().omni_bridge().set_pay_in_fee(asset_kind, dest_chain, 0);
            let hash = api
                .tx()
                .sign_and_submit_then_watch(&set_pay_in_fee, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();
        },
        SubstrateCommand::Balance(conf) => {
            // Query the account balance from the chain's `Balances` storage
            let account: AccountId32 = AccountId32::from_str(conf.account.as_str()).unwrap();

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
        },
        SubstrateCommand::PayIn(conf) => {
            let recipient_address = Vec::<u8>::from_hex(conf.dest_address.as_str()).expect("Failed to decode string");

            let request = litentry_rococo::runtime_types::pallet_omni_bridge::PayInRequest {
                asset: litentry_rococo::runtime_types::frame_support::traits::tokens::fungible::union_of::NativeOrWithId::Native,
                dest_chain: crate::litentry_rococo::runtime_types::core_primitives::omni::chain::ChainType::Ethereum(conf.ethereum_id),
                dest_account: recipient_address,
                amount: conf.amount,
            };

            let transfer_assets_call = litentry_rococo::tx().omni_bridge().pay_in(request);

            let hash = api
                .tx()
                .sign_and_submit_then_watch(&transfer_assets_call, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();
        },
        SubstrateCommand::FailedBridgeTx => {
            // Get the current finalized block number
            let latest_block = api.blocks().at_latest().await.unwrap();
            let mut current_block_hash = Some(latest_block.hash());

            let mut count = 0;

            // Scan the last 20 blocks for failed tx extrinsic events
            for _ in 0..20 {
                if let Some(block_hash) = current_block_hash {
                    let block = api.blocks().at(block_hash).await.unwrap();

                    // Fetch all events in the block
                    let events = block.events().await.unwrap();
                    for event in events.iter() {
                        let details = event.unwrap();
                        if let Ok(Some(ExtrinsicFailed { dispatch_error: DispatchError::Module(error), .. })) =
                            details.as_event::<ExtrinsicFailed>()
                        {
                            if error.index == 85 && error.error[0] == 10 {
                                count += 1;
                            }
                        }
                    }

                    // Get the parent hash to move to the previous block
                    current_block_hash = Some(block.header().parent_hash);
                }
            }
            println!("{}", count);
        },
    }
}
