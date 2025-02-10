use alloy::contract::{ContractInstance, Interface};
use std::str::FromStr;
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
use crate::HEIToken::HEITokenInstance;
use crate::LITToken::LITTokenInstance;
use alloy::dyn_abi::DynSolValue;
use alloy::hex::{decode, FromHex};
use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::{Address, Bytes, FixedBytes, B256, U256};
use alloy::providers::fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller};
use alloy::providers::{Identity, ProviderBuilder, RootProvider};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use alloy::transports::http::{Client, Http};
use clap::{Args, Subcommand};
use log::info;
use subxt_core::utils::AccountId32;

#[derive(Subcommand)]
pub enum EthereumCommand {
    SetupBridge(SetupBridgeCmdConf),
    AddRelayer(AddRelayerCmdConf),
    PayIn(PayInCmdConf),
    Balance(BalanceCmdConf),
}

#[derive(Args)]
pub struct BalanceCmdConf {
    #[arg(long, default_value = "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707")]
    token_address: String,
    #[arg(long, default_value = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8")]
    account: String,
}

#[derive(Args)]
// default values works for docker-compose setup
pub struct PayInCmdConf {
    #[arg(long, default_value = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY")]
    dest_address: String,
    #[arg(long, default_value = "100000000000000000000")]
    amount: String,
    #[arg(long, default_value = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d")]
    user_private_key: String,
    #[arg(long, default_value = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")]
    bridge_private_key: String,
    #[arg(long, default_value = "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9")]
    lit_token_address: String,
    #[arg(long, default_value = "0x5FbDB2315678afecb367f032d93F642f64180aa3")]
    bridge_address: String,
    #[arg(long, default_value = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512")]
    bridge_erc20_handler_address: String,
    #[arg(long, default_value = "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707")]
    hei_token_address: String,
}

#[derive(Args)]
pub struct SetupBridgeCmdConf {
    #[arg(long, default_value = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")]
    bridge_private_key: String,
    #[arg(long, default_value = "0x5FbDB2315678afecb367f032d93F642f64180aa3")]
    bridge_address: String,
    #[arg(long, default_value = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512")]
    bridge_erc20_handler_address: String,
    #[arg(long, default_value = "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707")]
    hei_token_address: String,
}

#[derive(Args)]
pub struct AddRelayerCmdConf {
    #[arg(long, default_value = "0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc")]
    relayer_address: String,
    #[arg(long, default_value = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")]
    bridge_private_key: String,
    #[arg(long, default_value = "0x5FbDB2315678afecb367f032d93F642f64180aa3")]
    bridge_address: String,
}

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Bridge,
    "../chainbridge-contracts/out/Bridge.sol/Bridge.json"
);
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    LITToken,
    "artifacts/LITToken.json"
);
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    HEIToken,
    "artifacts/HEI.json"
);

pub async fn handle(command: &EthereumCommand) {
    let rpc_url = "http://localhost:8545";
    // this is the first private key printed out by anvil during startup
    match command {
        EthereumCommand::PayIn(conf) => {
            let erc_20_handler_address = Address::from_slice(&decode(&conf.bridge_erc20_handler_address).unwrap());
            let hei_address = Address::from_slice(&decode(&conf.hei_token_address).unwrap());

            let user_signer =
                alloy::signers::local::PrivateKeySigner::from_str(conf.user_private_key.as_str()).unwrap();
            let address = user_signer.address();

            // transfer some tokens to user
            transfer_lit_to(&conf.bridge_private_key, address, &conf.amount, &conf.lit_token_address, rpc_url).await;
            // approve lit spending to HEI contract
            approve_lit_to(conf.user_private_key.as_str(), hei_address, &conf.amount, &conf.lit_token_address, rpc_url)
                .await;

            // approve HEI spending to ERC-20 handler contract
            approve_hei_to(
                conf.user_private_key.as_str(),
                erc_20_handler_address,
                &conf.amount,
                &conf.hei_token_address,
                rpc_url,
            )
            .await;

            // wrap some LIT tokens to HEI tokens
            wrap_to(conf.user_private_key.as_str(), address, &conf.amount, &conf.hei_token_address, rpc_url).await;

            // deposit on bridge instance
            bridge_deposit(
                conf.user_private_key.as_str(),
                &conf.amount,
                conf.dest_address.to_owned(),
                &conf.bridge_address,
                rpc_url,
            )
            .await;
        },
        EthereumCommand::AddRelayer(conf) => {
            add_relayer(
                &conf.bridge_private_key,
                &conf.bridge_address,
                Address::from_slice(&decode(&conf.relayer_address).unwrap()),
                rpc_url,
            )
            .await;
        },
        EthereumCommand::SetupBridge(conf) => {
            setup_bridge(
                &conf.bridge_private_key,
                &conf.bridge_address,
                &conf.bridge_erc20_handler_address,
                &conf.hei_token_address,
                rpc_url,
            )
            .await;
        },
        EthereumCommand::Balance(conf) => {
            let address = Address::from_str(&conf.account).unwrap();
            query_hei_token_amount(address, &conf.token_address, rpc_url).await;
        },
    }
}

async fn transfer_lit_to(
    bridge_owner_private_key: &str,
    address: Address,
    amount: &str,
    lit_token_address: &str,
    rpc_url: &str,
) {
    info!("Transferring LIT amount {} to {}", amount, address);
    let lit_token_instance = lit_token_instance(lit_token_address, bridge_owner_private_key, rpc_url).await;
    let transfer_builder = lit_token_instance.transfer(address, U256::from_str_radix(amount, 10).unwrap());
    transfer_builder.send().await.unwrap().watch().await.unwrap();
}

async fn wrap_to(owner_private_key: &str, address: Address, amount: &str, hei_token_address: &str, rpc_url: &str) {
    info!("Wrapping LIT amount {} to {}", amount, address);
    let hei_token_instance = hei_token_instance(hei_token_address, owner_private_key, rpc_url).await;
    let transfer_builder = hei_token_instance.depositFor(address, U256::from_str_radix(amount, 10).unwrap());
    transfer_builder.send().await.unwrap().watch().await.unwrap();
}

async fn query_hei_token_amount(address: Address, hei_token_address: &str, rpc_url: &str) {
    info!("Querying hei token amount on address {}", address);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(rpc_url.parse().unwrap());

    let artifact = include_str!("../artifacts/HEI.json");
    let json: serde_json::Value = serde_json::from_str(artifact).unwrap();

    let abi_value = json.get("abi").expect("Failed to get ABI from artifact");
    let abi = serde_json::from_str(&abi_value.to_string()).unwrap();

    let contract_instance =
        ContractInstance::new(Address::from_str(hei_token_address).unwrap(), provider, Interface::new(abi));

    let balance = contract_instance
        .function("balanceOf", &[DynSolValue::Address(address)])
        .unwrap()
        .call()
        .await
        .unwrap();
    println!("{}", balance.first().unwrap().as_uint().unwrap().0);
}

async fn approve_lit_to(
    owner_private_key: &str,
    spender: Address,
    amount: &str,
    lit_token_address: &str,
    rpc_url: &str,
) {
    info!("Approving LIT amount {} to {}", amount, spender);
    let lit_token_instance = lit_token_instance(lit_token_address, owner_private_key, rpc_url).await;
    let approve_builder = lit_token_instance.approve(spender, U256::from_str_radix(amount, 10).unwrap());
    approve_builder.send().await.unwrap().watch().await.unwrap();
}

async fn approve_hei_to(
    owner_private_key: &str,
    spender: Address,
    amount: &str,
    hei_token_address: &str,
    rpc_url: &str,
) {
    info!("Approving HEI amount {} to {}", amount, spender);
    let hei_token_instance = hei_token_instance(hei_token_address, owner_private_key, rpc_url).await;
    let approve_builder = hei_token_instance.approve(spender, U256::from_str_radix(amount, 10).unwrap());
    approve_builder.send().await.unwrap().watch().await.unwrap();
}
async fn add_relayer(by_private_key: &str, bridge_address: &str, relayer: Address, rpc_url: &str) {
    info!("Adding relayer {}", relayer);

    let bridge_instance = bridge_instance(bridge_address, by_private_key, rpc_url).await;
    let builder = bridge_instance.adminAddRelayer(relayer);
    builder.send().await.unwrap().watch().await.unwrap();
}

async fn setup_bridge(
    by_private_key: &str,
    bridge_address: &str,
    bridge_erc20_handler_address: &str,
    hei_token_address: &str,
    rpc_url: &str,
) {
    info!("Setting up bridge");
    let bridge_instance = bridge_instance(bridge_address, by_private_key, rpc_url).await;
    let resource_id = FixedBytes([
        158, 230, 223, 182, 26, 47, 185, 3, 223, 72, 124, 64, 22, 99, 130, 86, 67, 187, 130, 93, 65, 105, 94, 99, 223,
        138, 246, 22, 42, 177, 69, 166,
    ]);

    let builder = bridge_instance.adminSetResource(
        Address::from_hex(bridge_erc20_handler_address).unwrap(),
        resource_id,
        Address::from_hex(hei_token_address).unwrap(),
    );
    builder.send().await.unwrap().watch().await.unwrap();
    let builder_2 = bridge_instance.adminSetBurnable(
        Address::from_hex(bridge_erc20_handler_address).unwrap(),
        Address::from_hex(hei_token_address).unwrap(),
    );
    builder_2.send().await.unwrap().watch().await.unwrap();

    info!("Adding MINTER role to ERC20Handler on HEI contract instance");
    let hei_instance = hei_token_instance(hei_token_address, by_private_key, rpc_url).await;
    hei_instance
        .grantMinter(Address::from_hex(bridge_erc20_handler_address).unwrap())
        .send()
        .await
        .unwrap()
        .watch()
        .await
        .unwrap();
}

async fn bridge_deposit(by_private_key: &str, amount: &str, account: String, bridge_address: &str, rpc_url: &str) {
    info!("Bridging deposit");
    let bridge_instance = bridge_instance(bridge_address, by_private_key, rpc_url).await;
    let resource_id = FixedBytes([
        158, 230, 223, 182, 26, 47, 185, 3, 223, 72, 124, 64, 22, 99, 130, 86, 67, 187, 130, 93, 65, 105, 94, 99, 223,
        138, 246, 22, 42, 177, 69, 166,
    ]);
    // 0x + amount + address len + address (all 32 bytes padded)
    let amount = DynSolValue::Uint(U256::from_str_radix(amount, 10).unwrap(), 32).abi_encode();
    let account_id = AccountId32::from_str(account.as_str()).unwrap();
    let address_len = DynSolValue::Uint(U256::from(account_id.0.len()), 32).abi_encode();
    let address = DynSolValue::FixedBytes(B256::new(account_id.0), 32).abi_encode();

    let mut bytes = vec![];

    bytes.extend(amount);
    bytes.extend(address_len);
    bytes.extend(address);

    let call_data = Bytes::copy_from_slice(&bytes);
    let builder = bridge_instance.deposit(0, resource_id, call_data);
    builder.send().await.unwrap().watch().await.unwrap();
}

async fn bridge_instance(
    address: &str,
    private_key: &str,
    rpc_url: &str,
) -> crate::Bridge::BridgeInstance<
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
    let signer = PrivateKeySigner::from_slice(&decode(private_key).unwrap()).unwrap();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url.parse().unwrap());

    Bridge::new(Address::from_slice(&decode(address).unwrap()), provider)
}

async fn lit_token_instance(
    address: &str,
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
    let signer = PrivateKeySigner::from_slice(&decode(private_key).unwrap()).unwrap();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url.parse().unwrap());

    LITToken::new(Address::from_slice(&decode(address).unwrap()), provider)
}

async fn hei_token_instance(
    address: &str,
    private_key: &str,
    rpc_url: &str,
) -> HEITokenInstance<
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
    let signer = PrivateKeySigner::from_slice(&decode(private_key).unwrap()).unwrap();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url.parse().unwrap());

    HEITokenInstance::new(Address::from_slice(&decode(address).unwrap()), provider)
}
