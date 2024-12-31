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

use ethereum_listener::create_listener;
use ethereum_relayer::key_store::EthereumKeyStore;
use ethereum_relayer::EthereumRelayer;
use log::{error, info};
use std::thread;
use std::thread::JoinHandle;
use std::{fs, io::Write};
use substrate_listener::CustomConfig;
use substrate_relayer::key_store::SubstrateKeyStore;
use substrate_relayer::SubstrateRelayer;
use tokio::{runtime::Handle, sync::oneshot};

#[tokio::main]
async fn main() -> Result<(), ()> {
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

    fs::create_dir_all("data/").map_err(|e| {
        error!("Could not create data dir");
        ()
    })?;

    handles.push(sync_sepolia().unwrap());
    handles.push(sync_litentry_rococo().await.unwrap());

    for handle in handles {
        handle.join().unwrap()
    }

    Ok(())
}

async fn sync_litentry_rococo() -> Result<JoinHandle<()>, ()> {
    let (sub_stop_sender, sub_stop_receiver) = oneshot::channel();

    let key_store = EthereumKeyStore::new("data/ethereum_relayer_key.bin".to_string());

    let relayer = EthereumRelayer::new(
        "http://ethereum-node:8545",
        "0x5FbDB2315678afecb367f032d93F642f64180aa3",
        key_store,
    )
    .map_err(|e| log::error!("{:?}", e))?;

    info!(
        "Ethereum relayer address: {:?} ",
        hex::encode(relayer.get_address().as_slice())
    );

    let mut substrate_listener = substrate_listener::create_listener::<
        CustomConfig,
        substrate_listener::litentry_rococo::pallet_bridge::events::PaidIn,
    >(
        "litenty_rococo",
        Handle::current(),
        "ws://litentry-node:9944",
        Box::new(relayer),
        sub_stop_receiver,
    )
    .await?;

    Ok(thread::Builder::new()
        .name("litentry_rococo_sync".to_string())
        .spawn(move || substrate_listener.sync(0))
        .unwrap())
}

fn sync_sepolia() -> Result<JoinHandle<()>, ()> {
    let finalization_gap_blocks = 6;

    let key_store = SubstrateKeyStore::new("data/substrate_relayer_key.bin".to_string());

    let relayer: SubstrateRelayer<CustomConfig> =
        SubstrateRelayer::new("ws://litentry-node:9944", key_store);
    let (stop_sender, stop_receiver) = oneshot::channel();
    let mut eth_listener = create_listener(
        "sepolia",
        Handle::current(),
        "http://ethereum-node:8545",
        // "https://sepolia.infura.io/v3/26255715664b4092add78bac6d995719",
        vec![(
            // address of bridge smart contract
            "0x5FbDB2315678afecb367f032d93F642f64180aa3",
            Box::new(relayer),
        )],
        finalization_gap_blocks,
        stop_receiver,
    )?;

    Ok(thread::Builder::new()
        .name("sepolia".to_string())
        .spawn(move || eth_listener.sync(0))
        .unwrap())
}
