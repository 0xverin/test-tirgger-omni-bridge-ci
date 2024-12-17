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

use clap::Subcommand;
use std::str::FromStr;
use std::thread::sleep;
use subxt::ext::scale_value::stringify::custom_parsers::parse_hex;
use subxt::tx::Signer;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::dev;
use crate::litentry_rococo::runtime_types::rococo_parachain_runtime::RuntimeCall;
use crate::litentry_rococo::chain_bridge::Call;
use hex::FromHex;
use crate::litentry_rococo::runtime_types::pallet_bridge_common::AssetInfo;

#[subxt::subxt(runtime_metadata_path = "../artifacts/metadata.scale")]
pub mod litentry_rococo {}

#[derive(Subcommand)]
pub enum SubstrateCommand {
    SetupDevChainBridge
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
            let add_relayer_call = RuntimeCall::ChainBridge(
                Call::add_relayer{v: alice_signer.public_key().into()}
            );

            let add_relayer_sudo_call = litentry_rococo::tx()
                .sudo()
                .sudo(add_relayer_call);

            let hash = api
                .tx()
                .sign_and_submit_then_watch(&add_relayer_sudo_call, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let whitelist_chain_id = RuntimeCall::ChainBridge(
                Call::whitelist_chain{id: 0}
            );

            let whitelist_chain_id_sudo_call = litentry_rococo::tx()
                .sudo()
                .sudo(whitelist_chain_id);

            let hash = api
                .tx()
                .sign_and_submit_then_watch(&whitelist_chain_id_sudo_call, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let set_threshold_call = RuntimeCall::ChainBridge(
                Call::set_threshold{threshold: 1}
            );

            let set_threshold_sudo_call = litentry_rococo::tx()
                .sudo()
                .sudo(set_threshold_call);

            let hash = api
                .tx()
                .sign_and_submit_then_watch(&set_threshold_sudo_call, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();

            let resource_id = <[u8; 32]>::from_hex("6dbf3f9d61108d592cb424722ba78a9c2e786a3d1436508c9a02d7e48d70e41e").expect("Failed to decode hex string");
            let asset = AssetInfo {
                fee: 0, 
                asset: None
            };

            let set_resource_call = RuntimeCall::AssetsHandler(
                litentry_rococo::assets_handler::Call::set_resource {
                    resource_id,
                    asset
                }
            );

            let set_resource_sudo_call = litentry_rococo::tx()
                .sudo()
                .sudo(set_resource_call);

            let hash = api
                .tx()
                .sign_and_submit_then_watch(&set_resource_sudo_call, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();
        }
    }
}
