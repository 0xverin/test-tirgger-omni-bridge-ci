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

use serde::de::DeserializeOwned;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct BridgeConfig {
    pub listeners: Vec<Listener>,
    pub relayers: Vec<Relayer>,
}

impl BridgeConfig {
    pub fn get_listener_config<T: DeserializeOwned>(&self, index: usize) -> T {
        let listener = self.listeners.get(index).unwrap().clone();
        let config: T = serde_json::from_value(listener.config.clone()).unwrap();
        config
    }
}

#[derive(Clone, Deserialize)]
pub struct Listener {
    pub listener_type: String,
    pub id: String,
    pub relayers: Vec<String>,
    pub config: serde_json::Value,
}

impl Listener {
    pub fn to_specific_config<T: DeserializeOwned>(&self) -> T {
        let config: T = serde_json::from_value(self.config.clone()).unwrap();
        config
    }
}

#[derive(Deserialize)]
pub struct Relayer {
    pub relayer_type: String,
    pub id: String,
    pub config: serde_json::Value,
}

impl Relayer {
    pub fn to_specific_config<T: DeserializeOwned>(&self) -> T {
        let config: T = serde_json::from_value(self.config.clone()).unwrap();
        config
    }
}

// #[cfg(test)]
// pub mod tests {
//     use std::fs;
//
//     #[test]
//     pub fn deserialize_sample_config() {
//         let config = fs::read("../artifacts/bridge_config.json").unwrap();
//         let bridge_worker_config: Config = serde_json::from_slice(&config).unwrap();
//
//         assert_eq!(bridge_worker_config.listeners.len(), 2);
//         assert_eq!(bridge_worker_config.relayers.len(), 2);
//
//         assert_eq!(bridge_worker_config.listeners[0].id, "sepolia");
//         assert_eq!(bridge_worker_config.listeners[0].relayers[0], "rococo");
//         assert_eq!(bridge_worker_config.listeners[0].listener_type, "ethereum");
//
//         let sepolia_config: ethereum_listener::listener::ListenerConfig =
//             bridge_worker_config.get_listener_config(0);
//
//         assert_eq!(bridge_worker_config.listeners[1].id, "rococo");
//         assert_eq!(bridge_worker_config.listeners[1].relayers[0], "sepolia");
//         assert_eq!(bridge_worker_config.listeners[1].listener_type, "substrate");
//
//         let rococo_config: substrate_listener::listener::ListenerConfig =
//             bridge_worker_config.get_listener_config(1);
//
//         assert_eq!(bridge_worker_config.relayers[0].id, "sepolia");
//         assert_eq!(bridge_worker_config.relayers[0].relayer_type, "ethereum");
//
//         assert_eq!(bridge_worker_config.relayers[1].id, "rococo");
//         assert_eq!(bridge_worker_config.relayers[1].relayer_type, "substrate");
//     }
// }
