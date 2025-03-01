use crate::crypto::RsaCrypto;
use crate::error::CommonError;
use crate::user::{UserInfo, UserInfoRepository};
use accessory::Accessors;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, read_dir};
use std::future::Future;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
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
    user_info_storage: Arc<RwLock<HashMap<String, Arc<RwLock<UserInfo>>>>>,
}
impl FileSystemUserInfoRepository {
    pub async fn new<T, F, Fut>(
        refresh_interval: u64,
        user_repo_dir_path: &Path,
        prepare_additional_info: F,
    ) -> Result<Self, CommonError>
    where
        T: FsUserInfoContent + DeserializeOwned + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + Sync + 'static,
        F: Fn(Arc<RwLock<UserInfo>>, T) -> Fut + Clone + Send + Sync + 'static,
    {
        let user_info_storage = Arc::new(RwLock::new(HashMap::new()));
        {
            let user_info_storage = user_info_storage.clone();
            let user_repo_dir_path = user_repo_dir_path.to_owned();
            let init_signal = Arc::new(AtomicBool::new(false));
            {
                let init_signal = init_signal.clone();
                tokio::spawn(async move {
                    loop {
                        if let Err(e) = Self::fill_repo_storage(
                            prepare_additional_info.clone(),
                            user_info_storage.clone(),
                            &user_repo_dir_path,
                        )
                        .await
                        {
                            error!("Fail to build user repo:{e:?}");
                        } else {
                            init_signal.swap(true, Ordering::Relaxed);
                        }
                        sleep(Duration::from_secs(refresh_interval)).await;
                    }
                });
            }
            while !init_signal.load(Ordering::Relaxed) {
                sleep(Duration::from_millis(500)).await;
            }
        }

        Ok(Self { user_info_storage })
    }
    async fn fill_repo_storage<F, T, Fut>(
        prepare_additional_info: F,
        user_info_storage: Arc<RwLock<HashMap<String, Arc<RwLock<UserInfo>>>>>,
        user_repo_dir_path: &PathBuf,
    ) -> Result<(), CommonError>
    where
        T: FsUserInfoContent + DeserializeOwned + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + Sync + 'static,
        F: Fn(Arc<RwLock<UserInfo>>, T) -> Fut + Clone + Send + Sync + 'static,
    {
        let user_info_dir = read_dir(&user_repo_dir_path)?;
        for entry in user_info_dir.into_iter() {
            let Ok(entry) = entry else {
                error!("Fail to read user info directory [{user_repo_dir_path:?}]");
                continue;
            };
            let file_name = entry.file_name();
            let file_name = file_name.to_str();
            let Some(file_name) = file_name else {
                error!("Fail to read [{user_repo_dir_path:?}{file_name:?}].",);
                continue;
            };
            let file_name_parts = file_name.split('.').collect::<Vec<&str>>();
            if file_name_parts.len() < 2 {
                error!(
                    "Fail to read [{user_repo_dir_path:?}{file_name:?}] because of the file name is not in 2 parts",
                );
                continue;
            }
            let username = file_name_parts[0];
            let user_zip_file_path = user_repo_dir_path.join(format!("{}.zip", username));
            let user_zip_file = match File::open(user_zip_file_path) {
                Ok(user_zip_file) => user_zip_file,
                Err(e) => {
                    error!("Fail to read user zip file: {e:?}");
                    continue;
                }
            };
            let mut user_zip_archive = match ZipArchive::new(user_zip_file) {
                Ok(user_zip_archive) => user_zip_archive,
                Err(e) => {
                    error!("Fail to read user zip archive: {e:?}");
                    continue;
                }
            };
            let mut user_info_config_file_content = String::new();
            {
                let mut user_info_zip_file =
                    match user_zip_archive.by_name(FS_USER_INFO_CONFIG_FILE_NAME) {
                        Ok(user_info_file) => user_info_file,
                        Err(e) => {
                            error!("Fail to read user info file from zip archive: {e:?}");
                            continue;
                        }
                    };
                if let Err(e) =
                    user_info_zip_file.read_to_string(&mut user_info_config_file_content)
                {
                    error!("Fail to read user info file content from zip archive: {e:?}");
                    continue;
                };
            }
            let user_info_content = match toml::from_str::<T>(&user_info_config_file_content) {
                Ok(config) => config,
                Err(e) => {
                    error!("Fail to parse user info config file [{user_repo_dir_path:?}]: {e:?}");
                    continue;
                }
            };
            let mut public_key_file_content = String::new();
            {
                let mut public_key_zip_file = match user_zip_archive
                    .by_name(user_info_content.public_key_file_relative_path())
                {
                    Ok(user_info_file) => user_info_file,
                    Err(e) => {
                        error!("Fail to read public key file from zip archive: {e:?}");
                        continue;
                    }
                };
                if let Err(e) = public_key_zip_file.read_to_string(&mut public_key_file_content) {
                    error!("Fail to read public key file content from zip archive: {e:?}");
                    continue;
                };
            }
            let mut private_key_file_content = String::new();
            {
                let mut private_key_zip_file = match user_zip_archive
                    .by_name(user_info_content.private_key_file_relative_path())
                {
                    Ok(user_info_file) => user_info_file,
                    Err(e) => {
                        error!("Fail to read private key file from zip archive: {e:?}");
                        continue;
                    }
                };
                if let Err(e) = private_key_zip_file.read_to_string(&mut private_key_file_content) {
                    error!("Fail to read private key file content from zip archive: {e:?}");
                    continue;
                };
            }
            let Ok(rsa_crypto) = RsaCrypto::new(public_key_file_content, private_key_file_content)
            else {
                error!("Fail to create agent_user crypto for user: {username}.");
                continue;
            };
            let user_info = Arc::new(RwLock::new(UserInfo::new(rsa_crypto)));
            prepare_additional_info(user_info.clone(), user_info_content).await;
            let mut user_info_storage = user_info_storage.write().await;
            user_info_storage.insert(username.to_string(), user_info);
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl UserInfoRepository for FileSystemUserInfoRepository {
    async fn get_user(&self, username: &str) -> Result<Option<Arc<RwLock<UserInfo>>>, CommonError> {
        match self.user_info_storage.read().await.get(username) {
            None => Ok(None),
            Some(user_info) => Ok(Some(user_info.clone())),
        }
    }
    async fn get_single_user(
        &self,
    ) -> Result<Option<(String, Arc<RwLock<UserInfo>>)>, CommonError> {
        let user_info_storage = self.user_info_storage.read().await;
        let keys = user_info_storage.keys().collect::<Vec<&String>>();
        let first_key = match keys.first() {
            None => {
                return Ok(None);
            }
            Some(key) => *key,
        };
        let user = match user_info_storage.get(first_key) {
            None => {
                return Ok(None);
            }
            Some(user) => user,
        };
        Ok(Some((first_key.to_string(), user.clone())))
    }
}
