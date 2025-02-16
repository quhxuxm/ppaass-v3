pub mod repo;
use crate::crypto::RsaCrypto;
use crate::error::CommonError;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
#[derive(Debug)]
pub struct UserInfo {
    rsa_crypto: RsaCrypto,
    additional_info: HashMap<String, Arc<dyn Any + Send + Sync + 'static>>,
}

impl UserInfo {
    pub fn new(rsa_crypto: RsaCrypto) -> Self {
        Self {
            rsa_crypto,
            additional_info: Default::default(),
        }
    }

    pub fn rsa_crypto(&self) -> &RsaCrypto {
        &self.rsa_crypto
    }

    pub fn add_additional_info<T: Send + Sync + 'static>(&mut self, key: &str, value: T) {
        self.additional_info
            .insert(key.to_string(), Arc::new(value));
    }

    pub fn get_additional_info<T: Send + Sync + 'static>(&self, key: &str) -> Option<&T> {
        match self.additional_info.get(key) {
            None => None,
            Some(additional_info) => additional_info.downcast_ref::<T>(),
        }
    }
}

#[async_trait::async_trait]
pub trait UserInfoRepository {
    async fn get_user(&self, username: &str) -> Result<Option<Arc<RwLock<UserInfo>>>, CommonError>;
    async fn get_single_user(&self)
        -> Result<Option<(String, Arc<RwLock<UserInfo>>)>, CommonError>;
}
