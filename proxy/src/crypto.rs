use ppaass_common::crypto::{FileSystemRsaCryptoRepo, RsaCrypto, RsaCryptoRepository};
use ppaass_common::error::CommonError;
use std::sync::Arc;

#[derive(Debug)]
pub struct ForwardProxyRsaCryptoRepository {
    concrete_rsa_crypto_repo: FileSystemRsaCryptoRepo,
}
impl ForwardProxyRsaCryptoRepository {
    pub fn new(concrete_rsa_crypto_repo: FileSystemRsaCryptoRepo) -> Self {
        Self {
            concrete_rsa_crypto_repo,
        }
    }
}
impl RsaCryptoRepository for ForwardProxyRsaCryptoRepository {
    fn get_rsa_crypto(&self, key: &str) -> Result<Option<Arc<RsaCrypto>>, CommonError> {
        self.concrete_rsa_crypto_repo.get_rsa_crypto(key)
    }
}
