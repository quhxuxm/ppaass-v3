use crate::error::CommonError;
use hyper::body::Bytes;
pub use rsa::rand_core::OsRng;
use rsa::{
    pkcs8::{DecodePrivateKey, DecodePublicKey},
    Pkcs1v15Encrypt,
};
pub use rsa::{RsaPrivateKey, RsaPublicKey};

pub use rsa::pkcs8::EncodePrivateKey;
pub use rsa::pkcs8::EncodePublicKey;
pub use rsa::pkcs8::LineEnding;
use std::fmt::Debug;
use std::io::Read;
pub const DEFAULT_AGENT_PRIVATE_KEY_PATH: &str = "AgentPrivateKey.pem";
pub const DEFAULT_AGENT_PUBLIC_KEY_PATH: &str = "AgentPublicKey.pem";
pub const DEFAULT_PROXY_PRIVATE_KEY_PATH: &str = "ProxyPrivateKey.pem";
pub const DEFAULT_PROXY_PUBLIC_KEY_PATH: &str = "ProxyPublicKey.pem";

/// The util to do RSA encryption and decryption.
#[derive(Debug)]
pub struct RsaCrypto {
    /// The private used to do decryption
    private_key: RsaPrivateKey,
    /// The public used to do encryption
    public_key: RsaPublicKey,
}
impl RsaCrypto {
    pub fn new<A, B>(mut public_key_read: A, mut private_key_read: B) -> Result<Self, CommonError>
    where
        A: Read + Debug,
        B: Read + Debug,
    {
        let mut public_key_string = String::new();
        public_key_read.read_to_string(&mut public_key_string)?;
        let public_key = RsaPublicKey::from_public_key_pem(&public_key_string).map_err(|e| {
            CommonError::Rsa(format!("Fail to create agent_user public key: {e:?}"))
        })?;
        let mut private_key_string = String::new();
        private_key_read.read_to_string(&mut private_key_string)?;
        let private_key = RsaPrivateKey::from_pkcs8_pem(&private_key_string).map_err(|e| {
            CommonError::Rsa(format!("Fail to create agent_user private key: {e:?}"))
        })?;
        Ok(Self {
            public_key,
            private_key,
        })
    }

    /// Encrypt the target bytes with RSA public key
    pub fn encrypt(&self, target: &[u8]) -> Result<Bytes, CommonError> {
        let result = self
            .public_key
            .encrypt(&mut OsRng, Pkcs1v15Encrypt, target.as_ref())
            .map_err(|e| CommonError::Rsa(format!("Fail to encrypt with agent_user: {e:?}")))?;
        Ok(result.into())
    }

    /// Decrypt the target bytes with RSA private key
    pub fn decrypt(&self, target: &[u8]) -> Result<Bytes, CommonError> {
        let result = self
            .private_key
            .decrypt(Pkcs1v15Encrypt, target.as_ref())
            .map_err(|e| CommonError::Rsa(format!("Fail to decrypt with agent_user: {e:?}")))?;
        Ok(result.into())
    }
}
