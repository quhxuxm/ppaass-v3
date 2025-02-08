mod aes;
mod blowfish;
mod rsa;
use crate::config::RsaCryptoRepoConfig;
use crate::error::CommonError;
pub use aes::*;
pub use blowfish::*;
use hyper::body::Bytes;
use rand::random;
pub use rsa::*;
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::path::Path;
use std::sync::Arc;
use tracing::error;
#[inline(always)]
fn random_n_bytes<const N: usize>() -> Bytes {
    let random_n_bytes = random::<[u8; N]>();
    random_n_bytes.to_vec().into()
}

pub trait RsaCryptoRepository {
    fn get_rsa_crypto(&self, key: &str) -> Result<Option<Arc<RsaCrypto>>, CommonError>;
}

#[derive(Debug)]
pub struct FileSystemRsaCryptoRepo {
    cache: Arc<HashMap<String, Arc<RsaCrypto>>>,
}

impl FileSystemRsaCryptoRepo {
    pub fn new(repo_config: &impl RsaCryptoRepoConfig) -> Result<Self, CommonError> {
        let mut cache = HashMap::new();
        let rsa_dir = read_dir(repo_config.rsa_dir())?;
        rsa_dir.for_each(|entry| {
            let Ok(entry) = entry else {
                error!("Fail to read directory {:?}", repo_config.rsa_dir());
                return;
            };
            let user_token = entry.file_name();
            let user_token = user_token.to_str();
            let Some(user_token) = user_token else {
                error!(
                    "Fail to read {:?}{:?} directory because of user token not exist",
                    repo_config.rsa_dir(),
                    entry.file_name()
                );
                return;
            };
            let public_key_path = repo_config
                .rsa_dir()
                .join(user_token)
                .join(repo_config.public_key_name());
            let Ok(public_key_file) = File::open(&public_key_path) else {
                error!("Fail to read public key file: {public_key_path:?}.");
                return;
            };
            let private_key_path = repo_config
                .rsa_dir()
                .join(user_token)
                .join(repo_config.private_key_name());
            let private_key_path = Path::new(Path::new(&private_key_path));
            let Ok(private_key_file) = File::open(private_key_path) else {
                error!("Fail to read private key file :{private_key_path:?}.");
                return;
            };
            let Ok(rsa_crypto) = RsaCrypto::new(public_key_file, private_key_file) else {
                error!("Fail to create agent_rsa crypto for user: {user_token}.");
                return;
            };
            cache.insert(user_token.to_string(), Arc::new(rsa_crypto));
        });
        Ok(Self {
            cache: Arc::new(cache),
        })
    }
}

impl RsaCryptoRepository for FileSystemRsaCryptoRepo {
    fn get_rsa_crypto(&self, key: &str) -> Result<Option<Arc<RsaCrypto>>, CommonError> {
        match self.cache.get(key) {
            None => Ok(None),
            Some(val) => Ok(Some(val.clone())),
        }
    }
}
