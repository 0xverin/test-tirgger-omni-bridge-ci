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

use alloy::hex::decode;
use alloy::signers::k256::ecdsa::SigningKey;
use bridge_core::key_store::KeyStore;

// TODO: Can this read key from file and ask for password?
/// Generates and stores keys used by `EthereumRelayer`
pub struct EthereumKeyStore {
    path: String,
}

impl EthereumKeyStore {
    pub fn new(path: String) -> Self {
        let key = Self::generate_key().expect("Could not generate key");
        let store: EthereumKeyStore = Self { path };
        store.write(&key).expect("Could not write key");
        store
    }
}

impl KeyStore<SigningKey> for EthereumKeyStore {
    fn generate_key() -> Result<SigningKey, ()> {
        SigningKey::from_slice(
            &decode("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80").unwrap(),
        )
        .map_err(|_| ())
    }

    fn serialize(k: &SigningKey) -> Result<Vec<u8>, ()> {
        Ok(k.to_bytes().as_slice().to_vec())
    }

    fn deserialize(sealed: Vec<u8>) -> Result<SigningKey, ()> {
        Ok(SigningKey::from_slice(&sealed).map_err(|_| ())?)
    }

    fn path(&self) -> String {
        self.path.clone()
    }
}
