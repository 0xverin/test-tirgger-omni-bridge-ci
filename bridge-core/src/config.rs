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

use std::collections::HashSet;

use itertools::Itertools;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use thiserror::Error;

#[derive(Deserialize)]
pub struct BridgeConfig {
    pub listeners: Vec<Listener>,
    pub relayers: Vec<Relayer>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Listener ids are not unique")]
    ListenerIdNotUnique,
    #[error("Listener chain ids are not unique")]
    ListenerChainIdNotUnique,
    #[error("Listener relayer array is empty")]
    ListenerRelayersEmpty,
    #[error("Relayer assigned to listener is not defined")]
    ListenerRelayerNotDefined,
    #[error("Listener type is unknown")]
    ListenerTypeUnknown,
    #[error("Relayer ids are not unique")]
    RelayerIdNotUnique,
    #[error("Relayer destination ids are not unique")]
    RelayerDestinationIdNotUnique,
    #[error("Relayer is not used by any listener")]
    RelayerNotUsed,
    #[error("Relayer type is unknown")]
    RelayerTypeUnknown,
}

impl BridgeConfig {
    pub fn get_listener_config<T: DeserializeOwned>(&self, index: usize) -> T {
        let listener = self.listeners.get(index).unwrap().clone();
        let config: T = serde_json::from_value(listener.config.clone()).unwrap();
        config
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.check_listener_id_uniqueness()?;
        self.check_listener_type()?;
        self.check_listeners_relayer_arr_not_empty()?;
        self.check_relayer_id_uniqueness()?;
        self.check_relayer_type()?;
        self.check_relayer_destination_id_uniqueness()?;
        self.check_used_relayer_ids()?;

        Ok(())
    }

    fn check_listener_id_uniqueness(&self) -> Result<(), ConfigError> {
        if !self.listeners.iter().map(|listener| listener.id.as_str()).all_unique() {
            return Err(ConfigError::ListenerIdNotUnique);
        }
        Ok(())
    }

    fn check_listeners_relayer_arr_not_empty(&self) -> Result<(), ConfigError> {
        if self.listeners.iter().any(|listener| listener.relayers.is_empty()) {
            return Err(ConfigError::ListenerRelayersEmpty);
        }
        Ok(())
    }

    fn check_relayer_id_uniqueness(&self) -> Result<(), ConfigError> {
        if !self.relayers.iter().map(|relayer| relayer.id.as_str()).all_unique() {
            return Err(ConfigError::RelayerIdNotUnique);
        }
        Ok(())
    }

    fn check_relayer_destination_id_uniqueness(&self) -> Result<(), ConfigError> {
        if !self.relayers.iter().map(|relayer| relayer.destination_id.as_str()).all_unique() {
            return Err(ConfigError::RelayerDestinationIdNotUnique);
        }
        Ok(())
    }

    fn check_used_relayer_ids(&self) -> Result<(), ConfigError> {
        let mut relayers_used_by_listeners = HashSet::new();
        let mut relayers_defined = HashSet::new();

        for listener in &self.listeners {
            for relayer_id in &listener.relayers {
                relayers_used_by_listeners.insert(relayer_id);
            }
        }

        for relayer in &self.relayers {
            relayers_defined.insert(&relayer.id);
        }

        if !relayers_used_by_listeners
            .difference(&relayers_defined)
            .collect_vec()
            .is_empty()
        {
            return Err(ConfigError::ListenerRelayerNotDefined);
        }

        if !relayers_defined
            .difference(&relayers_used_by_listeners)
            .collect_vec()
            .is_empty()
        {
            return Err(ConfigError::RelayerNotUsed);
        }

        Ok(())
    }

    fn check_listener_type(&self) -> Result<(), ConfigError> {
        if self
            .listeners
            .iter()
            .any(|listener| listener.listener_type != "ethereum" && listener.listener_type != "substrate")
        {
            return Err(ConfigError::ListenerTypeUnknown);
        }
        Ok(())
    }

    fn check_relayer_type(&self) -> Result<(), ConfigError> {
        if self
            .relayers
            .iter()
            .any(|relayer| relayer.relayer_type != "ethereum" && relayer.relayer_type != "substrate")
        {
            return Err(ConfigError::RelayerTypeUnknown);
        }
        Ok(())
    }
}

#[derive(Clone, Deserialize)]
pub struct Listener {
    pub listener_type: String,
    pub id: String,
    pub relayers: Vec<String>,
    pub chain_id: u32,
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
    pub destination_id: String,
    pub id: String,
    pub config: serde_json::Value,
}

impl Relayer {
    pub fn to_specific_config<T: DeserializeOwned>(&self) -> T {
        let config: T = serde_json::from_value(self.config.clone()).unwrap();
        config
    }
}

#[cfg(test)]
pub mod tests {
    use crate::config::{BridgeConfig, ConfigError};
    use std::fs;

    use super::{Listener, Relayer};

    const LISTENER_1_ID: &str = "LISTENER_1";
    const LISTNER_TYPE: &str = "substrate";
    const CHAIN_0_ID: u32 = 0;
    const CHAIN_1_ID: u32 = 1;
    const RELAYER_1_ID: &str = "RELAYER_1";
    const RELAYER_2_ID: &str = "RELAYER_2";
    const RELAYER_TYPE: &str = "substrate";
    const DESTINATION_ID_1: &str = "DESTINATION_ID_1";
    const DESTINATION_ID_2: &str = "DESTINATION_ID_2";

    fn create_listener(id: &str, chain_id: u32, listener_type: &str, relayers: Vec<String>) -> Listener {
        Listener {
            id: id.to_string(),
            chain_id,
            listener_type: listener_type.to_string(),
            config: serde_json::Value::default(),
            relayers,
        }
    }

    fn create_relayer(id: &str, destination_id: &str, relayer_type: &str) -> Relayer {
        Relayer {
            id: id.to_string(),
            relayer_type: relayer_type.to_string(),
            destination_id: destination_id.to_string(),
            config: serde_json::Value::default(),
        }
    }

    #[test]
    pub fn validate_unique_listener_id() {
        let config = BridgeConfig {
            listeners: vec![
                create_listener(LISTENER_1_ID, CHAIN_0_ID, LISTNER_TYPE, vec![RELAYER_1_ID.to_string()]),
                create_listener(LISTENER_1_ID, CHAIN_1_ID, LISTNER_TYPE, vec![RELAYER_1_ID.to_string()]),
            ],
            relayers: vec![create_relayer(RELAYER_1_ID, DESTINATION_ID_1, RELAYER_TYPE)],
        };
        assert!(matches!(config.validate(), Err(ConfigError::ListenerIdNotUnique)))
    }

    #[test]
    pub fn validate_listener_type() {
        let config = BridgeConfig {
            listeners: vec![create_listener(LISTENER_1_ID, CHAIN_0_ID, "invalid", vec![RELAYER_1_ID.to_string()])],
            relayers: vec![create_relayer(RELAYER_1_ID, DESTINATION_ID_1, RELAYER_TYPE)],
        };
        assert!(matches!(config.validate(), Err(ConfigError::ListenerTypeUnknown)))
    }

    #[test]
    pub fn validate_listener_uses_only_defined_relayers() {
        let config = BridgeConfig {
            listeners: vec![create_listener(LISTENER_1_ID, CHAIN_0_ID, LISTNER_TYPE, vec![RELAYER_1_ID.to_string()])],
            relayers: vec![create_relayer(RELAYER_2_ID, DESTINATION_ID_1, RELAYER_TYPE)],
        };
        assert!(matches!(config.validate(), Err(ConfigError::ListenerRelayerNotDefined)))
    }

    #[test]
    pub fn validate_listener_relayers_not_empty() {
        let config = BridgeConfig {
            listeners: vec![create_listener(LISTENER_1_ID, CHAIN_0_ID, LISTNER_TYPE, vec![])],
            relayers: vec![],
        };
        assert!(matches!(config.validate(), Err(ConfigError::ListenerRelayersEmpty)))
    }

    #[test]
    pub fn validate_unique_relayer_id() {
        let config = BridgeConfig {
            listeners: vec![create_listener(LISTENER_1_ID, CHAIN_0_ID, LISTNER_TYPE, vec![RELAYER_1_ID.to_string()])],
            relayers: vec![
                create_relayer(RELAYER_1_ID, DESTINATION_ID_1, RELAYER_TYPE),
                create_relayer(RELAYER_1_ID, DESTINATION_ID_2, RELAYER_TYPE),
            ],
        };
        assert!(matches!(config.validate(), Err(ConfigError::RelayerIdNotUnique)))
    }

    #[test]
    pub fn validate_relayer_tyoe() {
        let config = BridgeConfig {
            listeners: vec![create_listener(LISTENER_1_ID, CHAIN_0_ID, LISTNER_TYPE, vec![RELAYER_1_ID.to_string()])],
            relayers: vec![create_relayer(RELAYER_1_ID, DESTINATION_ID_1, "invalid")],
        };
        assert!(matches!(config.validate(), Err(ConfigError::RelayerTypeUnknown)))
    }

    #[test]
    pub fn validate_unique_relayer_destination_id() {
        let config = BridgeConfig {
            listeners: vec![create_listener(LISTENER_1_ID, CHAIN_0_ID, LISTNER_TYPE, vec![RELAYER_1_ID.to_string()])],
            relayers: vec![
                create_relayer(RELAYER_1_ID, DESTINATION_ID_1, RELAYER_TYPE),
                create_relayer(RELAYER_2_ID, DESTINATION_ID_1, RELAYER_TYPE),
            ],
        };
        assert!(matches!(config.validate(), Err(ConfigError::RelayerDestinationIdNotUnique)))
    }

    #[test]
    pub fn validate_all_relayes_are_used() {
        let config = BridgeConfig {
            listeners: vec![create_listener(LISTENER_1_ID, CHAIN_0_ID, LISTNER_TYPE, vec![RELAYER_1_ID.to_string()])],
            relayers: vec![
                create_relayer(RELAYER_1_ID, DESTINATION_ID_1, RELAYER_TYPE),
                create_relayer(RELAYER_2_ID, DESTINATION_ID_2, RELAYER_TYPE),
            ],
        };
        assert!(matches!(config.validate(), Err(ConfigError::RelayerNotUsed)))
    }

    #[test]
    pub fn deserialize_sample_config() {
        let config = fs::read("../local/config.json").unwrap();
        let bridge_worker_config: BridgeConfig = serde_json::from_slice(&config).unwrap();

        assert_eq!(bridge_worker_config.listeners.len(), 3);
        assert_eq!(bridge_worker_config.relayers.len(), 3);

        assert_eq!(bridge_worker_config.listeners[0].id, "sepolia");
        assert_eq!(bridge_worker_config.listeners[0].relayers[0], "rococo");
        assert_eq!(bridge_worker_config.listeners[0].listener_type, "ethereum");

        let sepolia_config: ethereum_listener::listener::ListenerConfig = bridge_worker_config.get_listener_config(0);

        assert_eq!(sepolia_config.node_rpc_url, "http://ethereum-node:8545");
        assert_eq!(sepolia_config.bridge_contract_address, "0x5FbDB2315678afecb367f032d93F642f64180aa3");

        assert_eq!(bridge_worker_config.listeners[1].id, "ethereum-2");
        assert_eq!(bridge_worker_config.listeners[1].relayers[0], "rococo");
        assert_eq!(bridge_worker_config.listeners[1].listener_type, "ethereum");

        let ethereum_2_config: ethereum_listener::listener::ListenerConfig =
            bridge_worker_config.get_listener_config(1);

        assert_eq!(ethereum_2_config.node_rpc_url, "http://ethereum-2-node:8545");
        assert_eq!(ethereum_2_config.bridge_contract_address, "0x5FbDB2315678afecb367f032d93F642f64180aa3");

        assert_eq!(bridge_worker_config.listeners[2].id, "rococo");
        assert_eq!(bridge_worker_config.listeners[2].relayers[0], "sepolia");
        assert_eq!(bridge_worker_config.listeners[2].relayers[1], "ethereum-2");
        assert_eq!(bridge_worker_config.listeners[2].listener_type, "substrate");

        let rococo_config: substrate_listener::listener::ListenerConfig = bridge_worker_config.get_listener_config(2);

        assert_eq!(rococo_config.ws_rpc_endpoint, "ws://heima-node:9944");

        assert_eq!(bridge_worker_config.relayers[0].id, "sepolia");
        assert_eq!(bridge_worker_config.relayers[0].relayer_type, "ethereum");

        let sepolia_relayer_config: ethereum_relayer::RelayerConfig =
            bridge_worker_config.relayers[0].to_specific_config();

        assert_eq!(sepolia_relayer_config.node_rpc_url, "http://ethereum-node:8545");
        assert_eq!(sepolia_relayer_config.bridge_contract_address, "0x5FbDB2315678afecb367f032d93F642f64180aa3");

        assert_eq!(bridge_worker_config.relayers[1].id, "ethereum-2");
        assert_eq!(bridge_worker_config.relayers[1].relayer_type, "ethereum");

        let ethereum_2_relayer_config: ethereum_relayer::RelayerConfig =
            bridge_worker_config.relayers[1].to_specific_config();

        assert_eq!(ethereum_2_relayer_config.node_rpc_url, "http://ethereum-2-node:8545");
        assert_eq!(ethereum_2_relayer_config.bridge_contract_address, "0x5FbDB2315678afecb367f032d93F642f64180aa3");

        assert_eq!(bridge_worker_config.relayers[2].id, "rococo");
        assert_eq!(bridge_worker_config.relayers[2].relayer_type, "substrate");

        let rococo_relayer_config: substrate_relayer::RelayerConfig =
            bridge_worker_config.relayers[2].to_specific_config();

        assert_eq!(rococo_relayer_config.ws_rpc_endpoint, "ws://heima-node:9944");
    }
}
