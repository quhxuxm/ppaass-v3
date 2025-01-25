use crate::config::AgentConfig;
use crate::error::AgentError;
use ppaass_common::crypto::RsaCryptoRepository;
use std::sync::Arc;
pub struct Server<T>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    config: Arc<AgentConfig>,
    rsa_crypto_repo: Arc<T>,
}

impl<T> Server<T>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    pub fn new(config: Arc<AgentConfig>, rsa_crypto_repo: Arc<T>) -> Self {
        Self {
            config,
            rsa_crypto_repo,
        }
    }
    pub fn run(&self) -> Result<(), AgentError> {
        todo!()
    }
}
