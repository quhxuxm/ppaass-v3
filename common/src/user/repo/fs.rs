use crate::crypto::RsaCrypto;
use crate::error::CommonError;
use crate::user::{UserInfo, UserInfoRepository};
use accessory::Accessors;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::error;
pub const USER_INFO_ADDITION_INFO_EXPIRED_DATE_TIME: &str = "expired_date_time";
pub const USER_INFO_ADDITION_INFO_PROXY_SERVERS: &str = "proxy_servers";
pub const FS_USER_INFO_CONFIG_FILE_NAME: &str = "userinfo.toml";

pub trait FsUserInfoContent {
    fn public_key_file_relative_path(&self) -> &Path;
    fn private_key_file_relative_path(&self) -> &Path;
}

#[derive(Debug, Serialize, Deserialize, Accessors)]
pub struct FsAgentUserInfoContent {
    #[access(get(ty = &std::path::Path))]
    public_key_file_relative_path: PathBuf,
    #[access(get(ty = &std::path::Path))]
    private_key_file_relative_path: PathBuf,
    #[access(get)]
    proxy_servers: Vec<String>,
}

impl FsAgentUserInfoContent {
    pub fn new(
        proxy_servers: Vec<String>,
        public_key_file_relative_path: PathBuf,
        private_key_file_relative_path: PathBuf,
    ) -> Self {
        Self {
            proxy_servers,
            public_key_file_relative_path,
            private_key_file_relative_path,
        }
    }
}

impl FsUserInfoContent for FsAgentUserInfoContent {
    fn public_key_file_relative_path(&self) -> &Path {
        &self.public_key_file_relative_path
    }
    fn private_key_file_relative_path(&self) -> &Path {
        &self.private_key_file_relative_path
    }
}

#[derive(Debug, Serialize, Deserialize, Accessors)]
pub struct FsProxyUserInfoContent {
    #[access(get)]
    expired_date_time: Option<DateTime<Utc>>,
    #[access(get(ty = &std::path::Path))]
    public_key_file_relative_path: PathBuf,
    #[access(get(ty = &std::path::Path))]
    private_key_file_relative_path: PathBuf,
}

impl FsProxyUserInfoContent {
    pub fn new(
        expired_date_time: Option<DateTime<Utc>>,
        public_key_file_relative_path: PathBuf,
        private_key_file_relative_path: PathBuf,
    ) -> Self {
        Self {
            expired_date_time,
            public_key_file_relative_path,
            private_key_file_relative_path,
        }
    }
}

impl FsUserInfoContent for FsProxyUserInfoContent {
    fn public_key_file_relative_path(&self) -> &Path {
        &self.public_key_file_relative_path
    }
    fn private_key_file_relative_path(&self) -> &Path {
        &self.private_key_file_relative_path
    }
}

#[derive(Debug)]
pub struct FileSystemUserInfoRepository {
    user_info_storage: HashMap<String, Arc<UserInfo>>,
}
impl FileSystemUserInfoRepository {
    pub fn new<T, F>(
        user_repo_dir_path: &Path,
        mut prepare_additional_info: F,
    ) -> Result<Self, CommonError>
    where
        T: FsUserInfoContent + DeserializeOwned,
        F: FnMut(&mut UserInfo, T),
    {
        let mut user_info_storage = HashMap::new();
        let user_info_dir = read_dir(user_repo_dir_path)?;
        user_info_dir.for_each(|entry| {
            let Ok(entry) = entry else {
                error!("Fail to read user info directory [{:?}]", user_repo_dir_path);
                return;
            };
            let username = entry.file_name();
            let username = username.to_str();
            let Some(username) = username else {
                error!(
                    "Fail to read [{user_repo_dir_path:?}{:?}] directory because of username not exist",
                    entry.file_name()
                );
                return;
            };
            let user_info_config_file_path = user_repo_dir_path.join(username).join(FS_USER_INFO_CONFIG_FILE_NAME);
            let user_info_config_file_content = match std::fs::read_to_string(&user_info_config_file_path) {
                Ok(content) => content,
                Err(e) => {
                    error!("Fail to read user info config file [{:?}]: {e:?}", user_info_config_file_path);
                    return;
                }
            };
            let user_info_content = match toml::from_str::<T>(&user_info_config_file_content) {
                Ok(config) => config,
                Err(e) => {
                    error!("Fail to parse user info config file [{:?}]: {e:?}", user_repo_dir_path);
                    return;
                }
            };
            let public_key_path = user_repo_dir_path
                .join(username)
                .join(user_info_content.public_key_file_relative_path());
            let Ok(public_key_file) = File::open(&public_key_path) else {
                error!("Fail to read public key file: {public_key_path:?}.");
                return;
            };
            let private_key_path = user_repo_dir_path
                .join(username)
                .join(user_info_content.private_key_file_relative_path());
            let private_key_path = Path::new(Path::new(&private_key_path));
            let Ok(private_key_file) = File::open(private_key_path) else {
                error!("Fail to read private key file :{private_key_path:?}.");
                return;
            };
            let Ok(rsa_crypto) = RsaCrypto::new(public_key_file, private_key_file) else {
                error!("Fail to create agent_user crypto for user: {username}.");
                return;
            };
            let mut user_info = UserInfo::new(rsa_crypto);
            prepare_additional_info(&mut user_info, user_info_content);
            user_info_storage.insert(username.to_string(), Arc::new(user_info));
        });
        Ok(Self { user_info_storage })
    }
}
impl UserInfoRepository for FileSystemUserInfoRepository {
    fn get_user(&self, username: &str) -> Result<Option<Arc<UserInfo>>, CommonError> {
        match self.user_info_storage.get(username) {
            None => Ok(None),
            Some(user_info) => Ok(Some(user_info.clone())),
        }
    }

    fn get_single_user(&self) -> Result<Option<(String, Arc<UserInfo>)>, CommonError> {
        let keys = self.user_info_storage.keys().collect::<Vec<&String>>();
        let first_key = keys.first().ok_or(CommonError::Other(format!(
            "No users in the system: {:?}",
            keys
        )))?;
        let user = self
            .user_info_storage
            .get(*first_key)
            .ok_or(CommonError::Other(format!(
                "Can not find user by key: {first_key:?  }"
            )))?;
        Ok(Some((first_key.to_string(), user.clone())))
    }
}
