use crate::keystore::KeyStore as KeyStoreT;
use crate::rpc::error_code::*;
use crate::rpc::server::RpcContext;
use jsonrpsee::types::{ErrorObject, Params};
use jsonrpsee::RpcModule;
use log::{error, info};
use rsa::traits::PublicKeyParts;
use rsa::Oaep;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sha2::Sha256;
use sp_core::{ecdsa, keccak_256};
use std::sync::Arc;

impl<P: Serialize + std::fmt::Debug> SignedParams<P> {
    pub fn verify_signature(&self, signer: &[u8; 33]) -> bool {
        let msg = match serde_json::to_vec(&self.payload) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Could not serialize payload: {:?}", e);
                return false;
            },
        };

        let digest = keccak_256(&msg);

        ecdsa::Pair::verify_prehashed(
            &ecdsa::Signature::from_raw(self.signature),
            &digest,
            &ecdsa::Public::from_raw(*signer),
        )
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct SignedParams<P> {
    pub payload: P,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature: [u8; 65],
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct ImportRelayerKeyPayload {
    pub id: String,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub key: Vec<u8>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct ShieldingKey {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub n: Vec<u8>,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub e: Vec<u8>,
}

fn ensure_authorized_request<'a, P: Serialize + std::fmt::Debug>(
    params: &SignedParams<P>,
    signers: &[&[u8; 33]],
) -> Result<(), ErrorObject<'a>> {
    if signers.iter().any(|signer| params.verify_signature(signer)) {
        Ok(())
    } else {
        Err(ErrorObject::owned::<()>(UNAUTHORIZED_REQUEST_CODE, "Unauthorized request", None))
    }
}

// returns shielding key (RSA pubkey) of this signer
pub fn register_get_shielding_key<KeyStore: KeyStoreT>(module: &mut RpcModule<RpcContext<KeyStore>>) {
    module
        .register_async_method(
            "hm_getShieldingKey",
            |_params: Params, rpc_context: Arc<RpcContext<KeyStore>>, _| async move {
                let public_key = rpc_context.shielding_key.public_key();
                serde_json::to_value(ShieldingKey { n: public_key.n().to_bytes_le(), e: public_key.e().to_bytes_le() })
                    .unwrap()
            },
        )
        .unwrap();
}

pub fn register_import_relayer_key<KeyStore: KeyStoreT>(module: &mut RpcModule<RpcContext<KeyStore>>) {
    module
        .register_async_method(
            "hm_importRelayerKey",
            |params: Params, rpc_context: Arc<RpcContext<KeyStore>>, _| async move {
                let params = params.parse::<SignedParams<ImportRelayerKeyPayload>>()?;

                ensure_authorized_request(&params, &[&rpc_context.import_keystore_signer])?;

                let decrypted = rpc_context
                    .shielding_key
                    .private_key()
                    .decrypt(Oaep::new::<Sha256>(), &params.payload.key)
                    .map_err(|_| {
                        ErrorObject::owned::<()>(
                            SHIELDED_VALUE_DECRYPTION_ERROR_CODE,
                            "Shielded value decryption failed",
                            None,
                        )
                    })?;

                rpc_context
                    .keystore
                    .write()
                    .unwrap()
                    .set_key(&params.payload.id, decrypted)
                    .map_err(|e| ErrorObject::owned::<()>(KEYSTORE_WRITE_ERROR_CODE, e.to_string(), None))?;
                info!("Successfully imported relayer key with id {}", params.payload.id);
                Ok::<(), ErrorObject>(())
            },
        )
        .unwrap();
}
