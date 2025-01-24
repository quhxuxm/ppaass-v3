use crate::error::ServerError;
use crate::ServerConfig;
use ppaass_common::crypto::{RsaCrypto, RsaCryptoRepository};
use ppaass_common::error::CommonError;
pub struct ServerRsaCryptoRepo {}

impl ServerRsaCryptoRepo {
    pub fn new(server_config: &ServerConfig) -> Result<Self, ServerError> {
        todo!()
    }
}

impl RsaCryptoRepository for ServerRsaCryptoRepo {
    fn get_rsa_crypto(&self, key: &str) -> Result<Option<RsaCrypto>, CommonError> {
        todo!()
    }
}
