use super::*;
use log::*;
use sp_core::Pair;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

// The vault value (key) is opaque Vec<u8>, we should be able to tell
// if it's valid when initialising the relayer key, as we know the relayer
// type by then
pub struct LocalKeystore {
    path: PathBuf,
    vault: HashMap<String, Vec<u8>>,
}

impl LocalKeystore {
    // Initiate the keystore based on the given dir path:
    // It will read all files end with "<id>.bin", and store the content in the vault keyed by `id`
    pub fn open(path: PathBuf) -> Result<Self> {
        let mut vault: HashMap<String, Vec<u8>> = HashMap::new();

        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let file_path = entry.path();

            // Check if it's a file and ends with ".bin"
            if file_path.is_file() {
                if let Some(file_name) = file_path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        if file_name_str.ends_with(".bin") {
                            // Extract the prefix (e.g., "heima" from "heima.bin")
                            if let Some(prefix) = file_name_str.strip_suffix(".bin") {
                                let key = fs::read(&file_path)?;
                                vault.insert(prefix.to_string(), key);
                            }
                        }
                    }
                }
            }
        }

        info!("Open {:?} ok, get {} keys", path, vault.len());

        Ok(Self { path, vault })
    }

    pub fn seal_to_file(path: &PathBuf, key: Vec<u8>) -> Result<()> {
        let mut file = File::create(path)?;
        file.write_all(&key)?;
        file.flush()?;
        Ok(())
    }
}

impl KeyStore for LocalKeystore {
    fn set_key(&mut self, id: &str, key: Vec<u8>) -> Result<()> {
        self.vault.insert(id.to_string(), key.clone());
        let f = id.to_string() + ".bin";
        let path = self.path.as_path().join(f);
        Self::seal_to_file(&path, key)
    }

    fn sign_ecdsa(&self, id: &str, msg: &[u8; 32]) -> Result<sp_core::ecdsa::Signature> {
        let p = self
            .vault
            .get(id)
            .map(|k| sp_core::ecdsa::Pair::from_seed_slice(k).map_err(|_| Error::ParseEcdsaPair))
            .ok_or(Error::ParseEcdsaPair)??;
        Ok(p.sign_prehashed(msg))
    }

    fn sign_sr25519(&self, id: &str, msg: &[u8]) -> Result<sp_core::sr25519::Signature> {
        let p = self
            .vault
            .get(id)
            .map(|k| sp_core::sr25519::Pair::from_seed_slice(k).map_err(|_| Error::ParseSr25519Pair))
            .ok_or(Error::ParseSr25519Pair)??;
        Ok(p.sign(msg))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    // from subkey inspect '//Alice'
    const SR25519_SEED: &str = "e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a";
    const ECDSA_SEED: &str = "cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854";
    const MSG: [u8; 32] = [0u8; 32];

    const SR25519_SEED_2: &str = "398f0c28f98885e046333d4a41c19cee4c37368a9832c6502f6cfd182e2aef89";

    #[test]
    fn set_key_works() {
        // init

        println!("{}", hex::encode(MSG));
        fs::create_dir_all("data").unwrap();
        let mut keystore = LocalKeystore::open("data".into()).unwrap();
        assert_eq!(keystore.path, PathBuf::from_str("data").unwrap());
        assert!(keystore.vault.is_empty());

        keystore.set_key("ecdsa", hex::decode(ECDSA_SEED).unwrap()).unwrap();
        keystore.set_key("sr25519", hex::decode(SR25519_SEED).unwrap()).unwrap();

        assert_eq!(keystore.vault.len(), 2);
        assert_eq!(hex::encode(&keystore.vault["ecdsa"]), ECDSA_SEED);
        assert_eq!(hex::encode(&keystore.vault["sr25519"]), SR25519_SEED);

        assert!(PathBuf::from_str("data/ecdsa.bin").unwrap().is_file());
        assert!(PathBuf::from_str("data/sr25519.bin").unwrap().is_file());

        // re-read from same dir
        let mut keystore = LocalKeystore::open("data".into()).unwrap();
        assert_eq!(keystore.vault.len(), 2);
        assert_eq!(hex::encode(&keystore.vault["ecdsa"]), ECDSA_SEED);
        assert_eq!(hex::encode(&keystore.vault["sr25519"]), SR25519_SEED);

        // re-set to another key
        keystore.set_key("sr25519", hex::decode(SR25519_SEED_2).unwrap()).unwrap();

        // re-read and check if the change takes effect
        let keystore = LocalKeystore::open("data".into()).unwrap();
        assert_eq!(keystore.vault.len(), 2);
        assert_eq!(hex::encode(&keystore.vault["ecdsa"]), ECDSA_SEED);
        assert_eq!(hex::encode(&keystore.vault["sr25519"]), SR25519_SEED_2);

        fs::remove_dir_all("data").unwrap();
    }

    #[test]
    fn sign_works() {
        fs::create_dir_all("data").unwrap();
        let mut keystore = LocalKeystore::open("data".into()).unwrap();
        keystore.set_key("ecdsa", hex::decode(ECDSA_SEED).unwrap()).unwrap();
        keystore.set_key("sr25519", hex::decode(SR25519_SEED).unwrap()).unwrap();

        let sig = keystore.sign_sr25519("sr25519", &MSG).unwrap();
        assert!(sp_core::sr25519::Pair::verify(
            &sig,
            &MSG,
            &sp_core::sr25519::Public::from_str("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY").unwrap() // Alice
        ));

        let sig = keystore.sign_ecdsa("ecdsa", &MSG).unwrap();
        assert!(sp_core::ecdsa::Pair::verify_prehashed(
            &sig,
            &MSG,
            &sp_core::ecdsa::Public::from_str("KW39r9CJjAVzmkf9zQ4YDb2hqfAVGdRqn53eRqyruqpxAP5YL").unwrap() // Alice
        ));

        fs::remove_dir_all("data").unwrap();
    }
}
