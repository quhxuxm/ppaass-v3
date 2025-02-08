use ppaass_common::config::RsaCryptoRepoConfig;
use ppaass_common::crypto::{DEFAULT_AGENT_PUBLIC_KEY_PATH, DEFAULT_PROXY_PRIVATE_KEY_PATH};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyToolConfig {
    pub rsa_dir: PathBuf,
}

impl RsaCryptoRepoConfig for ProxyToolConfig {
    fn rsa_dir(&self) -> &Path {
        &self.rsa_dir
    }
    fn public_key_name(&self) -> &str {
        DEFAULT_AGENT_PUBLIC_KEY_PATH
    }
    fn private_key_name(&self) -> &str {
        DEFAULT_PROXY_PRIVATE_KEY_PATH
    }
}
