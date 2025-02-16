use crate::crypto::RsaCrypto;
use crate::error::CommonError;
use crate::user::{UserInfo, UserInfoRepository};
use accessory::Accessors;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use tracing::error;
use zip::ZipArchive;
pub const USER_INFO_ADDITION_INFO_EXPIRED_DATE_TIME: &str = "expired_date_time";
pub const USER_INFO_ADDITION_INFO_PROXY_SERVERS: &str = "proxy_servers";
pub const FS_USER_INFO_CONFIG_FILE_NAME: &str = "userinfo.toml";
pub trait FsUserInfoContent {
    fn public_key_file_relative_path(&self) -> &str;
    fn private_key_file_relative_path(&self) -> &str;
}
#[derive(Debug, Serialize, Deserialize, Accessors)]
pub struct FsAgentUserInfoContent {
    #[access(get)]
    public_key_file_relative_path: String,
    #[access(get)]
    private_key_file_relative_path: String,
    #[access(get)]
    proxy_servers: Vec<String>,
}
impl FsAgentUserInfoContent {
    pub fn new(
        proxy_servers: Vec<String>,
        public_key_file_relative_path: String,
        private_key_file_relative_path: String,
    ) -> Self {
        Self {
            proxy_servers,
            public_key_file_relative_path,
            private_key_file_relative_path,
        }
    }
}
impl FsUserInfoContent for FsAgentUserInfoContent {
    fn public_key_file_relative_path(&self) -> &str {
        &self.public_key_file_relative_path
    }
    fn private_key_file_relative_path(&self) -> &str {
        &self.private_key_file_relative_path
    }
}
#[derive(Debug, Serialize, Deserialize, Accessors)]
pub struct FsProxyUserInfoContent {
    #[access(get)]
    expired_date_time: Option<DateTime<Utc>>,
    #[access(get)]
    public_key_file_relative_path: String,
    #[access(get)]
    private_key_file_relative_path: String,
}
impl FsProxyUserInfoContent {
    pub fn new(
        expired_date_time: Option<DateTime<Utc>>,
        public_key_file_relative_path: String,
        private_key_file_relative_path: String,
    ) -> Self {
        Self {
            expired_date_time,
            public_key_file_relative_path,
            private_key_file_relative_path,
        }
    }
}
impl FsUserInfoContent for FsProxyUserInfoContent {
    fn public_key_file_relative_path(&self) -> &str {
        &self.public_key_file_relative_path
    }
    fn private_key_file_relative_path(&self) -> &str {
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
                error!(
                    "Fail to read user info directory [{:?}]",
                    user_repo_dir_path
                );
                return;
            };
            let file_name = entry.file_name();
            let file_name = file_name.to_str();
            let Some(file_name) = file_name else {
                error!("Fail to read [{user_repo_dir_path:?}{file_name:?}].",);
                return;
            };
            let file_name_parts = file_name.split('.').collect::<Vec<&str>>();
            if file_name_parts.len() < 2 {
                error!("Fail to read [{user_repo_dir_path:?}{file_name:?}] because of the file name is not in 2 parts",);
                return;
            }
            let username = file_name_parts[0];
            let user_zip_file_path = user_repo_dir_path
                .join(format!("{}.zip", username));
            let user_zip_file = match File::open(user_zip_file_path) {
                Ok(user_zip_file) => user_zip_file,
                Err(e) => {
                    error!("Fail to read user zip file: {e:?}");
                    return;
                }
            };
            let mut user_zip_archive = match ZipArchive::new(user_zip_file) {
                Ok(user_zip_archive) => user_zip_archive,
                Err(e) => {
                    error!("Fail to read user zip archive: {e:?}");
                    return;
                }
            };
            let mut user_info_config_file_content = String::new();
            {
                let mut user_info_zip_file = match user_zip_archive.by_name(FS_USER_INFO_CONFIG_FILE_NAME) {
                    Ok(user_info_file) => user_info_file,
                    Err(e) => {
                        error!("Fail to read user info file from zip archive: {e:?}");
                        return;
                    }
                };
                if let Err(e) = user_info_zip_file.read_to_string(&mut user_info_config_file_content) {
                    error!("Fail to read user info file content from zip archive: {e:?}");
                    return;
                };
            }
            let user_info_content = match toml::from_str::<T>(&user_info_config_file_content) {
                Ok(config) => config,
                Err(e) => {
                    error!(
                        "Fail to parse user info config file [{:?}]: {e:?}",
                        user_repo_dir_path
                    );
                    return;
                }
            };
            let mut public_key_file_content = String::new();
            {
                let mut public_key_zip_file = match user_zip_archive.by_name(user_info_content.public_key_file_relative_path()) {
                    Ok(user_info_file) => user_info_file,
                    Err(e) => {
                        error!("Fail to read public key file from zip archive: {e:?}");
                        return;
                    }
                };
                if let Err(e) = public_key_zip_file.read_to_string(&mut public_key_file_content) {
                    error!("Fail to read public key file content from zip archive: {e:?}");
                    return;
                };
            }
            let mut private_key_file_content = String::new();
            {
                let mut private_key_zip_file = match user_zip_archive.by_name(user_info_content.private_key_file_relative_path()) {
                    Ok(user_info_file) => user_info_file,
                    Err(e) => {
                        error!("Fail to read private key file from zip archive: {e:?}");
                        return;
                    }
                };
                if let Err(e) = private_key_zip_file.read_to_string(&mut private_key_file_content) {
                    error!("Fail to read private key file content from zip archive: {e:?}");
                    return;
                };
            }
            let Ok(rsa_crypto) = RsaCrypto::new(public_key_file_content, private_key_file_content) else {
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
