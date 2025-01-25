mod command;
mod config;
mod error;

mod tunnel;
use crate::command::Command;
use clap::Parser;
pub use config::*;
use ppaass_common::crypto::FileSystemRsaCryptoRepo;
use ppaass_common::init_logger;
use ppaass_common::server::{CommonServer, Server};

use crate::tunnel::handle_agent_connection;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::error;
const USER_AGENT_PUBLIC_KEY: &str = "AgentPublicKey.pem";
const USER_PROXY_PRIVATE_KEY: &str = "ProxyPrivateKey.pem";
const DEFAULT_CONFIG_FILE: &str = "resources/config.toml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = Command::parse();
    let config_file_path = command
        .config
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
    let config_file_content = read_to_string(config_file_path)?;
    let config = Arc::new(toml::from_str::<ProxyConfig>(&config_file_content)?);
    let log_dir = command.log_dir.unwrap_or(config.log_dir().clone());
    let _log_guard = init_logger(&log_dir, config.log_name_prefix(), config.max_log_level())?;
    let rsa_dir = command.rsa.unwrap_or(config.rsa_dir().clone());
    let rsa_crypto_repo = Arc::new(FileSystemRsaCryptoRepo::new(
        &rsa_dir,
        USER_AGENT_PUBLIC_KEY,
        USER_PROXY_PRIVATE_KEY,
    )?);
    let server = CommonServer::new(config, rsa_crypto_repo);
    if let Err(e) = server.run(handle_agent_connection) {
        error!("Fail to run proxy: {:?}", e);
    };
    Ok(())
}
