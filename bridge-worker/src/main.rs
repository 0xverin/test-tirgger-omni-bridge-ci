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

use crate::cli::*;
use crate::keystore::LocalKeystore;
use crate::rpc::methods::{ImportRelayerKeyPayload, SignedParams};
use crate::shielding_key::ShieldingKey;

use bridge_core::config::BridgeConfig;
use bridge_core::listener::{prepare_listener_context, ListenerContext, StartBlock};
use bridge_core::relay::Relayer;
use clap::Parser;
use ethereum_listener::create_listener;
use ethereum_listener::listener::ListenerConfig as EthereumListenerConfig;
use jsonrpsee_types::Id;
use log::*;
use metrics_exporter_prometheus::PrometheusBuilder;
use rand::rngs::OsRng;
use rand::Rng;
use rpc::server::start_server;
use rsa::traits::PublicKeyParts;
use rsa::{BigUint, Oaep, RsaPublicKey};
use serde_json::value::RawValue;
use sha2::Sha256;
use sp_core::{keccak_256, ByteArray, Pair};
use std::collections::HashMap;
use std::fs::create_dir;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use std::thread::JoinHandle;
use std::{fs, io::Write};
use std::{
    sync::{Arc, RwLock},
    thread,
};
use substrate_listener::listener::ListenerConfig as SubstrateListenerConfig;
use substrate_listener::CustomConfig;
use tokio::{runtime::Handle, signal, sync::oneshot};

mod cli;
mod keystore;
mod rpc;
mod shielding_key;

#[cfg(test)]
fn alice_signer() -> [u8; 33] {
    let key = sp_core::ecdsa::Pair::from_string("//Alice", None).unwrap();
    key.public().0
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let cli = Cli::parse();

    env_logger::builder()
        .format(|buf, record| {
            let ts = buf.timestamp_micros();
            writeln!(
                buf,
                "{} [{}][{}][{}]: {}",
                ts,
                record.level(),
                std::thread::current().name().unwrap_or("none"),
                record.target(),
                record.args(),
            )
        })
        .init();

    match &cli.command {
        Commands::Run(arg) => run(arg).await?,
        Commands::AwaitKeystoreImport(arg) => await_import(arg).await,
        Commands::GenerateAuthKey(arg) => generate_auth_key(arg),
        Commands::BuildKeystoreImport(arg) => build_import(arg),
    }

    Ok(())
}

async fn run(arg: &RunArgs) -> Result<(), ()> {
    let config_file = arg.config.clone();
    let keystore_dir = arg.keystore_dir.clone();

    let mut handles = vec![];

    let builder = PrometheusBuilder::new();

    let address = SocketAddr::from_str(&format!("0.0.0.0:{}", arg.metrics_port)).unwrap();
    builder
        .with_http_listener(address)
        .install()
        .expect("failed to install Prometheus recorder");

    let config: String = fs::read_to_string(config_file).unwrap();
    let config: BridgeConfig = serde_json::from_str(&config).unwrap();

    config.validate().map_err(|e| {
        error!("Config validation error: {:?}", e);
    })?;

    #[allow(clippy::type_complexity)]
    let mut relayers: HashMap<String, HashMap<String, Arc<Box<dyn Relayer<String>>>>> = HashMap::new();

    // substrate relayers
    let substrate_relayers: HashMap<String, Arc<Box<dyn Relayer<String>>>> =
        substrate_relayer::create_from_config::<CustomConfig>(keystore_dir.clone(), &config.relayers);
    relayers.insert("substrate".to_string(), substrate_relayers);

    // ethereum relayers
    let ethereum_relayers: HashMap<String, Arc<Box<dyn Relayer<String>>>> =
        ethereum_relayer::create_from_config(keystore_dir, &config).await;
    relayers.insert("ethereum".to_string(), ethereum_relayers);

    let mut start_blocks: HashMap<String, u64> = HashMap::new();

    arg.start_block
        .iter()
        .map(|s| {
            let start_block: StartBlock = s.try_into().unwrap();
            start_block
        })
        .for_each(|start_block| {
            start_blocks.insert(start_block.listener_id, start_block.block_num);
        });

    // start ethereum listeners
    let ethereum_listener_contexts: Vec<ListenerContext<EthereumListenerConfig>> =
        prepare_listener_context(&config, "ethereum", &relayers, &start_blocks);
    for ethereum_listener_context in ethereum_listener_contexts {
        handles.push(sync_ethereum(ethereum_listener_context).unwrap());
    }

    // start substrate listeners
    let substrate_listener_contexts: Vec<ListenerContext<SubstrateListenerConfig>> =
        prepare_listener_context(&config, "substrate", &relayers, &start_blocks);
    for substrate_listener_context in substrate_listener_contexts {
        // todo: remove unwrap ??
        handles.push(sync_substrate(substrate_listener_context).await.unwrap())
    }

    for handle in handles {
        handle.join().unwrap()
    }

    Ok(())
}

fn generate_auth_key(arg: &GenerateArgs) {
    println!("Generating auth key ...");
    let mut seed = [0u8; 32];
    OsRng.fill(&mut seed);
    let pair = sp_core::ecdsa::Pair::from_seed_slice(&seed).unwrap();

    if let Some(ref path) = arg.generate_path {
        if !Path::new(path).exists() {
            create_dir(path).unwrap();
        }
    }

    let auth_key_seed_path = arg
        .generate_path
        .as_ref()
        .map(|path| Path::new(path).join(AUTH_KEY_SEED_PATH))
        .unwrap_or(Path::new(AUTH_KEY_SEED_PATH).to_path_buf());
    let auth_key_pub_path = arg
        .generate_path
        .as_ref()
        .map(|path| Path::new(path).join(AUTH_KEY_PUB_PATH))
        .unwrap_or(Path::new(AUTH_KEY_PUB_PATH).to_path_buf());

    fs::write(&auth_key_seed_path, hex::encode(pair.seed().as_slice())).unwrap();
    fs::write(auth_key_pub_path, hex::encode(pair.public().as_slice())).unwrap();

    println!("Auth public key in hex: {}", hex::encode(pair.public().as_slice()));
    println!("Auth private key saved to file: {:?} ", auth_key_seed_path);
}

fn build_import(arg: &ImportArgs) {
    println!("Generating import relayer key command ...");
    let shielding_key = fs::read(arg.shielding_key_path.clone()).unwrap();
    let shielding_key: rpc::methods::ShieldingKey = serde_json::from_slice(shielding_key.as_slice()).unwrap();
    let shielding_key =
        RsaPublicKey::new(BigUint::from_bytes_le(&shielding_key.n), BigUint::from_bytes_le(&shielding_key.e)).unwrap();

    let auth_key = fs::read(arg.auth_key_path.clone()).unwrap();
    let auth_key = sp_core::ecdsa::Pair::from_seed_slice(&hex::decode(&auth_key).unwrap()).unwrap();

    build_import_internal(arg.substrate_id.clone(), arg.substrate_relayer_key_path.clone(), &shielding_key, &auth_key);
    build_import_internal(arg.ethereum_id.clone(), arg.ethereum_relayer_key_path.clone(), &shielding_key, &auth_key);
}

async fn sync_substrate(context: ListenerContext<SubstrateListenerConfig>) -> Result<JoinHandle<()>, ()> {
    let (_sub_stop_sender, sub_stop_receiver) = oneshot::channel();

    match context.config.chain.as_str() {
        "local" => {
            let mut listener = substrate_listener::create_local_listener::<CustomConfig>(
                &context.id,
                Handle::current(),
                &context.config,
                context.start_block,
                context.chain_id,
                context.relayers,
                sub_stop_receiver,
            )
            .await?;
            Ok(thread::Builder::new()
                .name(format!("{}_sync", &context.id).to_string())
                .spawn(move || {
                    let _ = listener.sync();
                })
                .unwrap())
        },
        "paseo" => {
            let mut listener = substrate_listener::create_paseo_listener::<CustomConfig>(
                &context.id,
                Handle::current(),
                &context.config,
                context.start_block,
                context.chain_id,
                context.relayers,
                sub_stop_receiver,
            )
            .await?;
            Ok(thread::Builder::new()
                .name(format!("{}_sync", &context.id).to_string())
                .spawn(move || {
                    let _ = listener.sync();
                })
                .unwrap())
        },
        "heima" => {
            let mut listener = substrate_listener::create_heima_listener::<CustomConfig>(
                &context.id,
                Handle::current(),
                &context.config,
                context.start_block,
                context.chain_id,
                context.relayers,
                sub_stop_receiver,
            )
            .await?;
            Ok(thread::Builder::new()
                .name(format!("{}_sync", &context.id).to_string())
                .spawn(move || {
                    let _ = listener.sync();
                })
                .unwrap())
        },
        _ => panic!("Unknown chain: {}", context.config.chain),
    }
}

fn sync_ethereum(context: ListenerContext<EthereumListenerConfig>) -> Result<JoinHandle<()>, ()> {
    let (_stop_sender, stop_receiver) = oneshot::channel();
    let mut eth_listener = create_listener(
        &context.id,
        Handle::current(),
        &context.config,
        context.start_block,
        context.chain_id,
        context.relayers,
        stop_receiver,
    )?;

    Ok(thread::Builder::new()
        .name(format!("{}_sync", &context.id).to_string())
        .spawn(move || {
            let _ = eth_listener.sync();
        })
        .unwrap())
}

fn build_import_internal(id: String, key_path: String, shielding_key: &RsaPublicKey, auth_key: &sp_core::ecdsa::Pair) {
    let relayer_key = fs::read(key_path).unwrap();
    let relayer_key = hex::decode(&relayer_key).unwrap();

    let shielded_relayer_key = shielding_key.encrypt(&mut OsRng, Oaep::new::<Sha256>(), &relayer_key).unwrap();

    let import_payload = ImportRelayerKeyPayload { id: id.clone(), key: shielded_relayer_key };
    let import_signature = auth_key
        .sign_prehashed(&keccak_256(&serde_json::to_vec(&import_payload).unwrap()))
        .to_raw();
    let import_signed_params = SignedParams { payload: import_payload, signature: import_signature };
    let import_request = jsonrpsee_types::RequestSer::owned(
        Id::Number(0),
        "hm_importRelayerKey",
        Some(RawValue::from_string(serde_json::to_string(&import_signed_params).unwrap()).unwrap()),
    );

    println!("\nImport {} relayer key cmd:", id);
    println!(
        "curl -X POST -H 'Content-Type: application/json' -d '{}' http://127.0.0.1:2000",
        serde_json::to_string(&import_request).unwrap()
    );
}

async fn await_import(arg: &AwaitImportArgs) {
    println!("Generating shielding key ...");
    let shielding_key = Arc::new(ShieldingKey::new());
    println!(
        "Shielding key: {}",
        serde_json::to_string(&rpc::methods::ShieldingKey {
            n: shielding_key.public_key().n().to_bytes_le(),
            e: shielding_key.public_key().e().to_bytes_le()
        })
        .unwrap()
    );

    let import_keystore_signer: [u8; 33] = hex::decode(fs::read(&arg.auth_pub_key_path).unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let keystore = Arc::new(RwLock::new(LocalKeystore::open(arg.keystore_dir.clone().into()).unwrap()));

    println!("Start server and wait for keystore import ...");

    start_server("0.0.0.0:2000", Handle::current(), import_keystore_signer, keystore, shielding_key).await;

    await_signal().await;
    println!("Bridge worker stopped");
}

async fn await_signal() {
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received Ctrl-C");
        },
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        },
    }
}
