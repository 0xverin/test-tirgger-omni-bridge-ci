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

use bridge_core::config::BridgeConfig;
use bridge_core::listener::{prepare_listener_context, ListenerContext};
use bridge_core::relay::Relayer;
use ethereum_listener::create_listener;
use ethereum_listener::listener::ListenerConfig as EthereumListenerConfig;
use log::error;
use std::collections::HashMap;
use std::thread::JoinHandle;
use std::{env, thread};
use std::{fs, io::Write};
use substrate_listener::listener::ListenerConfig as SubstrateListenerConfig;
use substrate_listener::CustomConfig;
use tokio::{runtime::Handle, sync::oneshot};

#[tokio::main]
async fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().collect();

    assert_eq!(args.len(), 2);
    let config_file = args.get(1).unwrap();

    let mut handles = vec![];

    env_logger::builder()
        .format(|buf, record| {
            let ts = buf.timestamp_micros();
            writeln!(
                buf,
                "{} [{}][{}]: {}",
                ts,
                record.level(),
                std::thread::current().name().unwrap_or("none"),
                record.args(),
            )
        })
        .init();

    fs::create_dir_all("data/").map_err(|_| {
        error!("Could not create data dir");
    })?;

    let config: String = fs::read_to_string(config_file).unwrap();
    let config: BridgeConfig = serde_json::from_str(&config).unwrap();

    let mut relayers: HashMap<String, HashMap<String, Box<dyn Relayer>>> = HashMap::new();

    // substrate relayers
    let substrate_relayers: HashMap<String, Box<dyn Relayer>> =
        substrate_relayer::create_from_config::<CustomConfig>(&config);
    relayers.insert("substrate".to_string(), substrate_relayers);

    // ethereum relayers
    let ethereum_relayers: HashMap<String, Box<dyn Relayer>> =
        ethereum_relayer::create_from_config(&config);
    relayers.insert("ethereum".to_string(), ethereum_relayers);

    // start ethereum listeners
    let ethereum_listener_contexts: Vec<ListenerContext<EthereumListenerConfig>> =
        prepare_listener_context(&config, "ethereum", &mut relayers);
    for ethereum_listener_context in ethereum_listener_contexts {
        handles.push(sync_ethereum(ethereum_listener_context).unwrap());
    }

    // start substrate listeners
    let substrate_listener_contexts: Vec<ListenerContext<SubstrateListenerConfig>> =
        prepare_listener_context(&config, "substrate", &mut relayers);
    for substrate_listener_context in substrate_listener_contexts {
        // todo: remove unwrap ??
        handles.push(
            sync_litentry_rococo(substrate_listener_context)
                .await
                .unwrap(),
        )
    }

    for handle in handles {
        handle.join().unwrap()
    }

    Ok(())
}

async fn sync_litentry_rococo(
    mut context: ListenerContext<SubstrateListenerConfig>,
) -> Result<JoinHandle<()>, ()> {
    let (_sub_stop_sender, sub_stop_receiver) = oneshot::channel();

    //todo: for now we assume there is only one relayer =]
    assert_eq!(context.relayers.len(), 1);

    let relayer: Box<dyn Relayer> = context.relayers.remove(0);

    let mut substrate_listener = substrate_listener::create_listener::<
        CustomConfig,
        substrate_listener::litentry_rococo::chain_bridge::events::FungibleTransfer,
    >(
        &context.id,
        Handle::current(),
        &context.config,
        relayer,
        sub_stop_receiver,
    )
    .await?;

    Ok(thread::Builder::new()
        .name(format!("{}_sync", &context.id).to_string())
        .spawn(move || substrate_listener.sync(0))
        .unwrap())
}

fn sync_ethereum(
    mut context: ListenerContext<EthereumListenerConfig>,
) -> Result<JoinHandle<()>, ()> {
    let finalization_gap_blocks = 6;

    assert_eq!(context.relayers.len(), 1);

    let relayer: Box<dyn Relayer> = context.relayers.remove(0);

    let (_stop_sender, stop_receiver) = oneshot::channel();
    let mut eth_listener = create_listener(
        &context.id,
        Handle::current(),
        &context.config,
        relayer,
        finalization_gap_blocks,
        stop_receiver,
    )?;

    Ok(thread::Builder::new()
        .name(context.id.to_string())
        .spawn(move || eth_listener.sync(0))
        .unwrap())
}
