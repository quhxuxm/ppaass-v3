mod aes;
mod rsa;
use crate::error::CommonError;
pub use aes::*;
pub use rsa::*;
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::path::Path;
use std::sync::Arc;
use tracing::error;
pub trait RsaCryptoRepository {
    fn get_rsa_crypto(&self, key: &str) -> Result<Option<Arc<RsaCrypto>>, CommonError>;
}

pub struct FileSystemRsaCryptoRepo {
    cache: Arc<HashMap<String, Arc<RsaCrypto>>>,
}

impl FileSystemRsaCryptoRepo {
    pub fn new(
        rsa_dir_path: &Path,
        public_key_file_name: &str,
        private_key_file_name: &str,
    ) -> Result<Self, CommonError> {
        let mut cache = HashMap::new();
        let rsa_dir = read_dir(rsa_dir_path)?;
        rsa_dir.for_each(|entry| {
            let Ok(entry) = entry else {
                error!("Fail to read directory {rsa_dir_path:?}");
                return;
            };
            let user_token = entry.file_name();
            let user_token = user_token.to_str();
            let Some(user_token) = user_token else {
                error!(
                    "Fail to read {rsa_dir_path:?}{:?} directory because of user token not exist",
                    entry.file_name()
                );
                return;
            };
            let public_key_path = rsa_dir_path.join(user_token).join(&public_key_file_name);
            let Ok(public_key_file) = File::open(&public_key_path) else {
                error!("Fail to read public key file: {public_key_path:?}.");
                return;
            };
            let private_key_path = rsa_dir_path.join(user_token).join(&private_key_file_name);
            let private_key_path = Path::new(Path::new(&private_key_path));
            let Ok(private_key_file) = File::open(private_key_path) else {
                error!("Fail to read private key file :{private_key_path:?}.");
                return;
            };
            let Ok(rsa_crypto) = RsaCrypto::new(public_key_file, private_key_file) else {
                error!("Fail to create rsa crypto for user: {user_token}.");
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
