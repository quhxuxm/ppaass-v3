use crate::config::ProxyToolConfig;
use crate::crypto::{generate_agent_key_pairs, generate_proxy_key_pairs};
use anyhow::{anyhow, Result};
use chrono::{TimeDelta, Utc};
use ppaass_common::crypto::{
    DEFAULT_AGENT_PRIVATE_KEY_PATH, DEFAULT_AGENT_PUBLIC_KEY_PATH, DEFAULT_PROXY_PRIVATE_KEY_PATH,
    DEFAULT_PROXY_PUBLIC_KEY_PATH,
};
use ppaass_common::generate_uuid;
use ppaass_common::user::repo::fs::{
    FsAgentUserInfoContent, FsProxyUserInfoContent, FS_USER_INFO_CONFIG_FILE_NAME,
};
use std::io::Write;
use std::net::SocketAddr;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use zip::write::SimpleFileOptions;
const DEFAULT_SEND_TO_AGENT_DIR: &str = "send_to_agent";
const DEFAULT_TEMP_DIR: &str = "temp";

pub struct GenerateUserHandlerArgument {
    pub username: String,
    pub temp_dir: Option<PathBuf>,
    pub agent_rsa_dir: Option<PathBuf>,
    pub expire_after_days: Option<i64>,
    pub proxy_servers: Vec<String>,
}

fn generate_agent(temp_user_dir: &Path, arg: &GenerateUserHandlerArgument) -> Result<()> {
    println!(
        "Begin to generate agent user info configuration file for: {}",
        &arg.username
    );
    let default_send_to_agent_dir = PathBuf::from(DEFAULT_SEND_TO_AGENT_DIR);
    if !default_send_to_agent_dir.exists() {
        std::fs::create_dir_all(&default_send_to_agent_dir)?;
    }
    let proxy_public_key_file_content = std::fs::read_to_string(
        temp_user_dir
            .join(&arg.username)
            .join(DEFAULT_PROXY_PUBLIC_KEY_PATH),
    )?;
    let agent_private_key_file_content = std::fs::read_to_string(
        temp_user_dir
            .join(&arg.username)
            .join(DEFAULT_AGENT_PRIVATE_KEY_PATH),
    )?;

    if arg.proxy_servers.is_empty() {
        eprintln!("Proxy servers should not be empty.");
        return Err(anyhow!("Proxy servers should not be empty."));
    }
    for proxy_server in arg.proxy_servers.iter() {
        if let Err(e) = SocketAddr::from_str(&proxy_server) {
            eprintln!("Failed to parse proxy server: {proxy_server}");
            return Err(anyhow!("Fail to parse proxy server address: {e:?}"));
        }
    }

    let agent_user_info = FsAgentUserInfoContent::new(
        arg.proxy_servers.clone(),
        DEFAULT_PROXY_PUBLIC_KEY_PATH.to_string(),
        DEFAULT_AGENT_PRIVATE_KEY_PATH.to_string(),
    );
    let agent_user_info_config_file_content = toml::to_string(&agent_user_info)?;
    let zip_file_path = default_send_to_agent_dir.join(format!("{}.zip", &arg.username));
    let zip_file_path = Path::new(&zip_file_path);
    let zip_file = std::fs::File::create(zip_file_path)?;
    let mut zip_file_writer = zip::ZipWriter::new(zip_file);
    let zi_file_options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip_file_writer.start_file(FS_USER_INFO_CONFIG_FILE_NAME, zi_file_options)?;
    zip_file_writer.write_all(agent_user_info_config_file_content.as_bytes())?;
    zip_file_writer.start_file(DEFAULT_PROXY_PUBLIC_KEY_PATH, zi_file_options)?;
    zip_file_writer.write_all(proxy_public_key_file_content.as_bytes())?;
    zip_file_writer.start_file(DEFAULT_AGENT_PRIVATE_KEY_PATH, zi_file_options)?;
    zip_file_writer.write_all(agent_private_key_file_content.as_bytes())?;
    println!("Success write agent user configuration to: {zip_file_path:?}",);
    Ok(())
}

pub fn generate_proxy(
    config: &ProxyToolConfig,
    temp_user_dir: &Path,
    arg: &GenerateUserHandlerArgument,
) -> Result<()> {
    println!(
        "Begin to generate proxy user info configuration file for: {}",
        &arg.username
    );
    let proxy_user_dir = config.user_dir();
    if !proxy_user_dir.exists() {
        std::fs::create_dir_all(&proxy_user_dir)?;
    }
    let proxy_private_key_file_content = std::fs::read_to_string(
        temp_user_dir
            .join(&arg.username)
            .join(DEFAULT_PROXY_PRIVATE_KEY_PATH),
    )?;

    let agent_public_key_file_content = std::fs::read_to_string(
        temp_user_dir
            .join(&arg.username)
            .join(DEFAULT_AGENT_PUBLIC_KEY_PATH),
    )?;

    let expired_date_time = match arg.expire_after_days {
        None => None,
        Some(days) => Some(Utc::now().add(TimeDelta::days(days))),
    };

    let proxy_user_info = FsProxyUserInfoContent::new(
        expired_date_time,
        DEFAULT_AGENT_PUBLIC_KEY_PATH.to_string(),
        DEFAULT_PROXY_PRIVATE_KEY_PATH.to_string(),
    );
    let proxy_user_info_config_file_content = toml::to_string(&proxy_user_info)?;

    let zip_file_path = proxy_user_dir.join(format!("{}.zip", &arg.username));
    let zip_file_path = Path::new(&zip_file_path);
    let zip_file = std::fs::File::create(zip_file_path)?;
    let mut zip_file_writer = zip::ZipWriter::new(zip_file);
    let zi_file_options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip_file_writer.start_file(FS_USER_INFO_CONFIG_FILE_NAME, zi_file_options)?;
    zip_file_writer.write_all(proxy_user_info_config_file_content.as_bytes())?;
    zip_file_writer.start_file(DEFAULT_PROXY_PRIVATE_KEY_PATH, zi_file_options)?;
    zip_file_writer.write_all(proxy_private_key_file_content.as_bytes())?;
    zip_file_writer.start_file(DEFAULT_AGENT_PUBLIC_KEY_PATH, zi_file_options)?;
    zip_file_writer.write_all(agent_public_key_file_content.as_bytes())?;
    println!("Success write proxy user configuration to: {zip_file_path:?}",);
    Ok(())
}

pub fn generate_user(config: &ProxyToolConfig, arg: GenerateUserHandlerArgument) -> Result<()> {
    let default_tmp_dir = PathBuf::from(DEFAULT_TEMP_DIR);
    let temp_dir = arg.temp_dir.as_ref().unwrap_or_else(|| &default_tmp_dir);
    let temp_user_dir = temp_dir.join(generate_uuid());
    println!(
        "Begin to generate RSA key for [{}] in [{:?}]",
        arg.username, temp_user_dir
    );
    generate_proxy_key_pairs(&temp_user_dir, &arg.username)?;
    generate_agent_key_pairs(&temp_user_dir, &arg.username)?;
    generate_proxy(config, &temp_user_dir, &arg)?;
    generate_agent(&temp_user_dir, &arg)?;
    Ok(())
}
