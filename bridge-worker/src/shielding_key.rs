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

use rsa::{RsaPrivateKey, RsaPublicKey};

pub struct ShieldingKey {
    key: RsaPrivateKey,
}

impl ShieldingKey {
    pub fn new() -> Self {
        // create new
        let mut rng = rand::thread_rng();
        let bits = 3072;
        let key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
        Self { key }
    }

    #[cfg(test)]
    pub fn init_with(key: RsaPrivateKey) -> Self {
        Self { key }
    }

    pub fn public_key(&self) -> RsaPublicKey {
        self.key.to_public_key()
    }

    pub fn private_key(&self) -> &RsaPrivateKey {
        &self.key
    }
}
