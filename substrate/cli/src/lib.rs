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
use subxt::ext::scale_value::stringify::custom_parsers::parse_hex;
use subxt::tx::Signer;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::ecdsa::dev;

#[subxt::subxt(runtime_metadata_path = "../artifacts/rococo-bridge.scale")]
pub mod litentry_rococo {}

#[derive(Subcommand)]
pub enum SubstrateCommand {
    PayIn { amount: String },
    AddRelayer { account: String },
}

pub async fn handle(command: &SubstrateCommand) {
    let rpc_url = "ws://localhost:9944";
    let alice_signer = dev::alice();
    log::info!("Alice: {:?}", alice_signer);

    match command {
        SubstrateCommand::PayIn { amount } => {
            let call = litentry_rococo::tx()
                .pallet_bridge()
                .pay_in(10, [0; 32].to_vec());
            let api = OnlineClient::<PolkadotConfig>::from_insecure_url(rpc_url)
                .await
                .unwrap();
            let hash = api
                .tx()
                .sign_and_submit_then_watch(&call, &alice_signer, Default::default())
                .await
                .unwrap();
            hash.wait_for_finalized().await.unwrap();
        }
        SubstrateCommand::AddRelayer { account } => {
            let call = litentry_rococo::tx()
                .pallet_bridge()
                .add_relayer(AccountId32::from_str(account).unwrap());
            let api = OnlineClient::<PolkadotConfig>::from_insecure_url(rpc_url)
                .await
                .unwrap();
            let hash = api
                .tx()
                .sign_and_submit_then_watch(&call, &alice_signer, Default::default())
                .await
                .unwrap();

            hash.wait_for_finalized().await.unwrap();
        }
    }
}
