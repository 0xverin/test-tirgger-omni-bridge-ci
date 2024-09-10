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

use crate::Bridge::BridgeInstance;
use crate::LITToken::LITTokenInstance;
use alloy::hex::decode;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::{Address, Bytes, U256};
use alloy::providers::fillers::{
    ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::{Identity, ProviderBuilder, RootProvider};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use alloy::transports::http::{Client, Http};
use clap::Subcommand;
use log::info;

#[derive(Subcommand)]
pub enum EthereumCommand {
    Full {},
    Transfer { to: String, amount: String },
    PayIn { amount: String },
    Approve { to: String, amount: String },
    AddRelayer { address: String },
}

static LIT_ERC20_OWNER_PRIVATE_KEY: &str =
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
static BRIDGE_OWNER_PRIVATE_KEY: &str = LIT_ERC20_OWNER_PRIVATE_KEY;

static BRIDGE_ADDRESS: &str = "0x5FbDB2315678afecb367f032d93F642f64180aa3";

static USER_PRIVATE_KEY: &str =
    "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

static USER_ADDRESS: &str = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Bridge,
    "../bridge-contracts/out/Bridge.sol/Bridge.json"
);
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    LITToken,
    "artifacts/LITToken.json"
);

pub async fn handle(command: &EthereumCommand) {
    let rpc_url = "http://localhost:8545";
    // this is the first private key printed out by anvil during startup
    let user_address = Address::from_slice(&decode(USER_ADDRESS).unwrap());
    let bridge_address = Address::from_slice(&decode(BRIDGE_ADDRESS).unwrap());
    match command {
        EthereumCommand::Full {} => {
            // transfer some tokens to user
            transfer_lit_to(user_address, "100", rpc_url).await;

            // approve bridge to take 10 LIT from user
            approve_lit_to(USER_PRIVATE_KEY, bridge_address, "10", rpc_url).await;

            // this will be always the same as long as we use the same private key for deployment and this will be the first contract deployed by that address
            bridge_pay_in(USER_PRIVATE_KEY, "10", rpc_url).await;
        }
        EthereumCommand::Transfer { to, amount } => {
            // transfer some tokens to user
            transfer_lit_to(Address::from_slice(&decode(to).unwrap()), amount, rpc_url).await;
        }
        EthereumCommand::PayIn { amount } => {
            // this will be always the same as long as we use the same private key for deployment and this will be the first contract deployed by that address
            bridge_pay_in(USER_PRIVATE_KEY, amount, rpc_url).await;
        }
        EthereumCommand::Approve { to, amount } => {
            approve_lit_to(
                USER_PRIVATE_KEY,
                Address::from_slice(&decode(to).unwrap()),
                amount,
                rpc_url,
            )
            .await;
        }
        EthereumCommand::AddRelayer { address } => {
            add_relayer(
                BRIDGE_OWNER_PRIVATE_KEY,
                Address::from_slice(&decode(address).unwrap()),
                rpc_url,
            )
            .await;
        }
    }
}

async fn transfer_lit_to(address: Address, amount: &str, rpc_url: &str) {
    info!("Transferring LIT amount {} to {}", amount, address);
    let lit_token_instance = lit_token_instance(LIT_ERC20_OWNER_PRIVATE_KEY, rpc_url).await;
    let transfer_builder =
        lit_token_instance.transfer(address, U256::from_str_radix(amount, 10).unwrap());
    transfer_builder
        .send()
        .await
        .unwrap()
        .watch()
        .await
        .unwrap();
}

async fn approve_lit_to(owner_private_key: &str, spender: Address, amount: &str, rpc_url: &str) {
    info!("Approving LIT amount {} to {}", amount, spender);
    let lit_token_instance = lit_token_instance(owner_private_key, rpc_url).await;
    let approve_builder =
        lit_token_instance.approve(spender, U256::from_str_radix(amount, 10).unwrap());
    approve_builder.send().await.unwrap().watch().await.unwrap();
}

async fn bridge_pay_in(by_private_key: &str, amount: &str, rpc_url: &str) {
    info!("Calling Bridge PayIn amount {}", amount);
    let bridge_instance = bridge_instance(by_private_key, rpc_url).await;
    let builder = bridge_instance.payIn(U256::from_str_radix(amount, 10).unwrap(), Bytes::new());
    builder.send().await.unwrap().watch().await.unwrap();
}

async fn add_relayer(by_private_key: &str, relayer: Address, rpc_url: &str) {
    info!("Adding relayer {}", relayer);
    let bridge_instance = bridge_instance(by_private_key, rpc_url).await;
    let builder = bridge_instance.addRelayer(relayer);
    builder.send().await.unwrap().watch().await.unwrap();
}

async fn bridge_instance(
    private_key: &str,
    rpc_url: &str,
) -> BridgeInstance<
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
    Ethereum,
> {
    let bridge_smart_contract_address = "0x5FbDB2315678afecb367f032d93F642f64180aa3";

    let signer = PrivateKeySigner::from_slice(&decode(private_key).unwrap()).unwrap();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url.parse().unwrap());

    Bridge::new(
        Address::from_slice(&decode(bridge_smart_contract_address).unwrap()),
        provider,
    )
}

async fn lit_token_instance(
    private_key: &str,
    rpc_url: &str,
) -> LITTokenInstance<
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
    Ethereum,
> {
    let lit_token_smart_contract_address = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512";

    let signer = PrivateKeySigner::from_slice(&decode(private_key).unwrap()).unwrap();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url.parse().unwrap());

    LITToken::new(
        Address::from_slice(&decode(lit_token_smart_contract_address).unwrap()),
        provider,
    )
}
