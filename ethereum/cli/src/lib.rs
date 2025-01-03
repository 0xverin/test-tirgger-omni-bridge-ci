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
use alloy::providers::fillers::{
    ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::{Identity, ProviderBuilder, RootProvider};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use alloy::transports::http::{Client, Http};
use clap::Subcommand;
use log::info;
use subxt_core::utils::AccountId32;

#[derive(Subcommand)]
pub enum EthereumCommand {
    Bridge { amount: String, to: String },
    AddRelayer { address: String },
    SetupBridge,
}

static LIT_ERC20_OWNER_PRIVATE_KEY: &str =
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
static BRIDGE_OWNER_PRIVATE_KEY: &str = LIT_ERC20_OWNER_PRIVATE_KEY;

static BRIDGE_ADDRESS: &str = "0x5FbDB2315678afecb367f032d93F642f64180aa3";

static BRIDGE_ERC_20_HANDLER_ADDRESS: &str = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512";

static LIT_ADDRESS: &str = "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9";
static HEI_ADDRESS: &str = "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707";

static USER_PRIVATE_KEY: &str =
    "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

static USER_ADDRESS: &str = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";

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
    let user_address = Address::from_slice(&decode(USER_ADDRESS).unwrap());
    let hei_address = Address::from_slice(&decode(HEI_ADDRESS).unwrap());
    let erc_20_handler_address =
        Address::from_slice(&decode(BRIDGE_ERC_20_HANDLER_ADDRESS).unwrap());
    match command {
        EthereumCommand::Bridge { amount, to } => {
            // transfer some tokens to user
            transfer_lit_to(user_address, amount, rpc_url).await;
            // approve lit spending to HEI contract
            approve_lit_to(USER_PRIVATE_KEY, hei_address, amount, rpc_url).await;

            // approve HEI spending to ERC-20 handler contract
            approve_hei_to(USER_PRIVATE_KEY, erc_20_handler_address, amount, rpc_url).await;

            // wrap some LIT tokens to HEI tokens
            wrap_to(USER_PRIVATE_KEY, user_address, amount, rpc_url).await;

            // deposit on bridge instance
            bridge_deposit(USER_PRIVATE_KEY, amount, to.to_owned(), rpc_url).await;
        }
        EthereumCommand::AddRelayer { address } => {
            add_relayer(
                BRIDGE_OWNER_PRIVATE_KEY,
                Address::from_slice(&decode(address).unwrap()),
                rpc_url,
            )
            .await;
        }
        EthereumCommand::SetupBridge => {
            setup_bridge(BRIDGE_OWNER_PRIVATE_KEY, rpc_url).await;
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

async fn wrap_to(owner_private_key: &str, address: Address, amount: &str, rpc_url: &str) {
    info!("Wrapping LIT amount {} to {}", amount, address);
    let hei_token_instance = hei_token_instance(owner_private_key, rpc_url).await;
    let transfer_builder =
        hei_token_instance.depositFor(address, U256::from_str_radix(amount, 10).unwrap());
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

async fn approve_hei_to(owner_private_key: &str, spender: Address, amount: &str, rpc_url: &str) {
    info!("Approving HEI amount {} to {}", amount, spender);
    let hei_token_instance = hei_token_instance(owner_private_key, rpc_url).await;
    let approve_builder =
        hei_token_instance.approve(spender, U256::from_str_radix(amount, 10).unwrap());
    approve_builder.send().await.unwrap().watch().await.unwrap();
}
async fn add_relayer(by_private_key: &str, relayer: Address, rpc_url: &str) {
    info!("Adding relayer {}", relayer);

    let bridge_instance = bridge_instance(by_private_key, rpc_url).await;
    let builder = bridge_instance.adminAddRelayer(relayer);
    builder.send().await.unwrap().watch().await.unwrap();
}

async fn setup_bridge(by_private_key: &str, rpc_url: &str) {
    info!("Setting up bridge");
    let bridge_instance = bridge_instance(by_private_key, rpc_url).await;
    let resource_id = FixedBytes([0; 32]);

    let builder = bridge_instance.adminSetResource(
        Address::from_hex(BRIDGE_ERC_20_HANDLER_ADDRESS).unwrap(),
        resource_id,
        Address::from_hex(HEI_ADDRESS).unwrap(),
    );
    builder.send().await.unwrap().watch().await.unwrap();
    let builder_2 = bridge_instance.adminSetBurnable(
        Address::from_hex(BRIDGE_ERC_20_HANDLER_ADDRESS).unwrap(),
        Address::from_hex(HEI_ADDRESS).unwrap(),
    );
    builder_2.send().await.unwrap().watch().await.unwrap();
}

async fn bridge_deposit(by_private_key: &str, amount: &str, account: String, rpc_url: &str) {
    info!("Bridging deposit");
    let bridge_instance = bridge_instance(by_private_key, rpc_url).await;
    let resource_id = FixedBytes([0; 32]);
    // 0x + amount + address len + address (all 32 bytes padded)
    let amount = DynSolValue::Uint(U256::from_str_radix(amount, 10).unwrap(), 32).abi_encode();
    let account_id = AccountId32::from_str(account.as_str()).unwrap();
    let address_len = DynSolValue::Uint(U256::from(account_id.0.len()), 32).abi_encode();
    // todo: use user specified address here
    let address =
        DynSolValue::FixedBytes(B256::new(account_id.0.try_into().unwrap()), 32).abi_encode();

    let mut bytes = vec![];

    bytes.extend(amount);
    bytes.extend(address_len);
    bytes.extend(address);

    let call_data = Bytes::copy_from_slice(&bytes);
    let builder = bridge_instance.deposit(0, resource_id, call_data);
    builder.send().await.unwrap().watch().await.unwrap();
}

async fn bridge_instance(
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
    let bridge_smart_contract_address = BRIDGE_ADDRESS;

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
    let lit_token_smart_contract_address = LIT_ADDRESS;

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

async fn hei_token_instance(
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
    let hei_token_smart_contract_address = HEI_ADDRESS;

    let signer = PrivateKeySigner::from_slice(&decode(private_key).unwrap()).unwrap();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url.parse().unwrap());

    HEITokenInstance::new(
        Address::from_slice(&decode(hei_token_smart_contract_address).unwrap()),
        provider,
    )
}
