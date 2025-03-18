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
use clap::{Args, Parser, Subcommand};
use ethereum_cli::EthereumCommand;
use std::fs;
use substrate_cli::SubstrateCommand;
// !!!Only for dev purposes!!!

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Args)]
pub struct CheckConfigArgs {
    path: String,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(subcommand)]
    Ethereum(EthereumCommand),
    #[command(subcommand)]
    Substrate(SubstrateCommand),
    CheckConfig(CheckConfigArgs),
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    env_logger::builder().init();
    let cli = Cli::parse();

    match &cli.command {
        Some(Command::Ethereum(ethereum_command)) => {
            ethereum_cli::handle(ethereum_command).await;
        },
        Some(Command::Substrate(substrate_command)) => {
            substrate_cli::handle(substrate_command).await;
        },
        Some(Command::CheckConfig(args)) => {
            let config: String = fs::read_to_string(&args.path).unwrap();
            let config: BridgeConfig = serde_json::from_str(&config).unwrap();
            config.validate().unwrap();
            println!("Config ok.");
        },
        _ => println!("No command specified!"),
    }

    Ok(())
}
