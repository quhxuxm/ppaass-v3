use crate::config::ProxyToolConfig;
use crate::crypto::{generate_agent_key_pairs, generate_proxy_key_pairs};
use anyhow::{anyhow, Result};
use chrono::{TimeDelta, Utc};
use ppaass_common::crypto::{
    DEFAULT_AGENT_PRIVATE_KEY_PATH, DEFAULT_AGENT_PUBLIC_KEY_PATH, DEFAULT_PROXY_PRIVATE_KEY_PATH,
    DEFAULT_PROXY_PUBLIC_KEY_PATH,
};
use ppaass_common::generate_uuid;
use ppaass_common::user::repo::fs::{FileSystemUserInfoConfig, FS_USER_INFO_CONFIG_FILE_NAME};
use std::net::SocketAddr;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const DEFAULT_SEND_TO_AGENT_DIR: &str = "send_to_agent";
const DEFAULT_TEMP_DIR: &str = "temp";

pub struct GenerateUserHandlerArgument {
    pub username: String,
    pub temp_dir: Option<PathBuf>,
    pub agent_rsa_dir: Option<PathBuf>,
    pub expire_after_days: Option<i64>,
    pub proxy_servers: Option<Vec<String>>,
}
pub fn generate_user(config: &ProxyToolConfig, arg: GenerateUserHandlerArgument) -> Result<()> {
    let temp_dir = &arg
        .temp_dir
        .unwrap_or(Path::new(DEFAULT_TEMP_DIR).to_owned());
    let temp_user_dir = temp_dir.join(generate_uuid());
    println!(
        "Begin to generate RSA key for [{}] in [{:?}]",
        arg.username, temp_user_dir
    );
    generate_proxy_key_pairs(&temp_user_dir, &arg.username)?;
    generate_agent_key_pairs(&temp_user_dir, &arg.username)?;
    let proxy_user_dir = config.user_dir().join(&arg.username);
    println!(
        "Begin to copy generated RSA key into proxy user folder for [{}] in [{:?}]",
        arg.username, proxy_user_dir
    );
    if !proxy_user_dir.exists() {
        std::fs::create_dir_all(&proxy_user_dir)?;
    }
    std::fs::copy(
        temp_user_dir
            .join(&arg.username)
            .join(DEFAULT_PROXY_PRIVATE_KEY_PATH),
        proxy_user_dir.join(DEFAULT_PROXY_PRIVATE_KEY_PATH),
    )?;
    std::fs::copy(
        temp_user_dir
            .join(&arg.username)
            .join(DEFAULT_AGENT_PUBLIC_KEY_PATH),
        proxy_user_dir.join(DEFAULT_AGENT_PUBLIC_KEY_PATH),
    )?;
    println!(
        "Begin to generate proxy user info configuration file for: {}",
        &arg.username
    );
    let expired_date_time = match arg.expire_after_days {
        None => None,
        Some(days) => Some(Utc::now().add(TimeDelta::days(days))),
    };
    let proxy_user_info = FileSystemUserInfoConfig {
        username: arg.username.clone(),
        expired_date_time,
        proxy_servers: None,
        description: None,
        public_key_file_relative_path: PathBuf::from(DEFAULT_AGENT_PUBLIC_KEY_PATH),
        private_key_file_relative_path: PathBuf::from(DEFAULT_PROXY_PRIVATE_KEY_PATH),
    };
    let proxy_user_info_config_file_content = toml::to_string(&proxy_user_info)?;
    let proxy_user_info_config_file_path = proxy_user_dir.join(FS_USER_INFO_CONFIG_FILE_NAME);
    std::fs::write(
        &proxy_user_info_config_file_path,
        &proxy_user_info_config_file_content,
    )?;
    println!(
        "Success write proxy user info configuration file to: {proxy_user_info_config_file_path:?}",
    );

    let agent_user_dir = &arg
        .agent_rsa_dir
        .unwrap_or(PathBuf::from(DEFAULT_SEND_TO_AGENT_DIR))
        .join(&arg.username);
    println!(
        "Begin to copy generated RSA key into agent user folder for [{}] in [{:?}]",
        arg.username, agent_user_dir
    );
    if !agent_user_dir.exists() {
        std::fs::create_dir_all(&agent_user_dir)?;
    }
    std::fs::copy(
        temp_user_dir
            .join(&arg.username)
            .join(DEFAULT_PROXY_PUBLIC_KEY_PATH),
        agent_user_dir.join(DEFAULT_PROXY_PUBLIC_KEY_PATH),
    )?;
    std::fs::copy(
        temp_user_dir
            .join(&arg.username)
            .join(DEFAULT_AGENT_PRIVATE_KEY_PATH),
        agent_user_dir.join(DEFAULT_AGENT_PRIVATE_KEY_PATH),
    )?;

    println!(
        "Begin to generate agent user info configuration file for: {}",
        &arg.username
    );

    if let Some(proxy_servers) = &arg.proxy_servers {
        for proxy_server in proxy_servers {
            if let Err(e) = SocketAddr::from_str(&proxy_server) {
                eprintln!("Failed to parse proxy server: {proxy_server}");
                return Err(anyhow!("Fail to parse proxy server address: {e:?}"));
            }
        }
    }

    let agent_user_info = FileSystemUserInfoConfig {
        username: arg.username,
        expired_date_time: None,
        description: None,
        proxy_servers: arg.proxy_servers,
        public_key_file_relative_path: PathBuf::from(DEFAULT_PROXY_PUBLIC_KEY_PATH),
        private_key_file_relative_path: PathBuf::from(DEFAULT_AGENT_PRIVATE_KEY_PATH),
    };
    let agent_user_info_config_file_content = toml::to_string(&agent_user_info)?;
    let agent_user_info_config_file_path = agent_user_dir.join(FS_USER_INFO_CONFIG_FILE_NAME);
    std::fs::write(
        &agent_user_info_config_file_path,
        &agent_user_info_config_file_content,
    )?;
    println!(
        "Success write agent user info configuration file to: {proxy_user_info_config_file_path:?}",
    );

    Ok(())
}
