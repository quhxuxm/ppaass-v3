mod aes;
mod rsa;
use crate::error::CommonError;
pub use aes::*;
pub use rsa::*;
pub trait RsaCryptoRepository {
    fn get_rsa_crypto(&self, key: &str) -> Result<Option<RsaCrypto>, CommonError>;
}
