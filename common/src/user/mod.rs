pub mod repo;
use crate::crypto::RsaCrypto;
use crate::error::CommonError;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

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

    pub fn get_additional_info<T: Send + Sync + 'static>(&self, key: &str) -> Option<Arc<T>> {
        match self.additional_info.get(key) {
            None => None,
            Some(additional_info) => {
                let val = additional_info.downcast_ref::<Arc<T>>();
                match val {
                    None => None,
                    Some(val) => Some(val.clone()),
                }
            }
        }
    }
}

pub trait UserInfoRepository {
    fn get_user(&self, username: &str) -> Result<Option<Arc<UserInfo>>, CommonError>;
}
