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

use clap::{Args, Parser, Subcommand};

pub const SHIELDING_KEY_PATH: &str = "shielding_key.bin";
pub const AUTH_KEY_SEED_PATH: &str = "auth_key_seed.bin";
pub const AUTH_KEY_PUB_PATH: &str = "auth_key_pub.bin";
pub const SUBSTRATE_RELAYER_KEY_PATH: &str = "substrate_relayer_key.bin";
pub const ETHEREUM_RELAYER_KEY_PATH: &str = "ethereum_relayer_key.bin";

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Run(RunArgs),
    /// Wait for keystore import
    AwaitKeystoreImport(AwaitImportArgs),
    /// Generates curl command to import keystore
    BuildKeystoreImport(ImportArgs),
    /// Generates new ECDSA JSON-RPC auth key for keystore import
    GenerateAuthKey,
}

#[derive(Args)]
pub struct RunArgs {
    #[arg(short, long, default_value = "keystore", value_name = "keystore folder path")]
    pub keystore_dir: String,

    #[arg(short, long, default_value = "config.json", value_name = "bridge config file path")]
    pub config: String,
}

#[derive(Args)]
pub struct ImportArgs {
    #[arg(long)]
    pub substrate_id: String,

    #[arg(long)]
    pub ethereum_id: String,

    #[arg(long, default_value = SUBSTRATE_RELAYER_KEY_PATH)]
    pub substrate_relayer_key_path: String,

    #[arg(long, default_value = ETHEREUM_RELAYER_KEY_PATH)]
    pub ethereum_relayer_key_path: String,

    #[arg(long, default_value = AUTH_KEY_SEED_PATH)]
    pub auth_key_path: String,

    #[arg(long, default_value = SHIELDING_KEY_PATH)]
    pub shielding_key_path: String,
}

#[derive(Args)]
pub struct AwaitImportArgs {
    #[arg(short, long, default_value = "keystore", value_name = "keystore folder path")]
    pub keystore_dir: String,

    #[arg(long, default_value = AUTH_KEY_PUB_PATH)]
    pub auth_pub_key_path: String,
}