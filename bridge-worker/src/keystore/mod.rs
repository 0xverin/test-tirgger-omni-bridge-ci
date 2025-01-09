mod local;
pub use local::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Failed to parse as ECDSA pair")]
    ParseEcdsaPair,

    #[error("Failed to parse as SR25519 pair")]
    ParseSr25519Pair,

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Sync + Send + 'static>),
}

pub type Result<T> = std::result::Result<T, Error>;

#[allow(unused)]
pub trait KeyStore: Send + Sync + 'static {
    /// set the opaque private key by `id`
    fn set_key(&mut self, id: &str, key: Vec<u8>) -> Result<()>;

    /// Sign the `msg` with the ecdsa private key identified by `id`
    /// `msg` needs to be pre-hashed to 32 bytes
    fn sign_ecdsa(&self, id: &str, msg: &[u8; 32]) -> Result<sp_core::ecdsa::Signature>;

    /// Sign the `msg` with the sr25519 private key identified by `id`
    fn sign_sr25519(&self, id: &str, msg: &[u8]) -> Result<sp_core::sr25519::Signature>;
}
