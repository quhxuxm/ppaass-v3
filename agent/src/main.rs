use crate::command::Command;
use crate::config::AgentConfig;
use clap::Parser;
use ppaass_common::crypto::FileSystemRsaCryptoRepo;
use ppaass_common::init_logger;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
mod command;
mod config;
mod error;
mod server;

const USER_SERVER_PUBLIC_KEY: &str = "ProxyPublicKey.pem";
const USER_AGENT_PRIVATE_KEY: &str = "AgentPrivateKey.pem";
const DEFAULT_CONFIG_FILE: &str = "resources/config.toml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = Command::parse();
    let config_file_path = command
        .config
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
    let config_file_content = read_to_string(config_file_path)?;
    let config = Arc::new(toml::from_str::<AgentConfig>(&config_file_content)?);
    let log_dir = command.log_dir.unwrap_or(config.log_dir().clone());
    let _log_guard = init_logger(&log_dir, config.log_name_prefix(), config.max_log_level())?;
    let rsa_dir = command.rsa.unwrap_or(config.rsa_dir().clone());
    let rsa_crypto_repo = Arc::new(FileSystemRsaCryptoRepo::new(
        &rsa_dir,
        USER_SERVER_PUBLIC_KEY,
        USER_AGENT_PRIVATE_KEY,
    )?);
    let server = Server::new(config, rsa_crypto_repo);
    if let Err(e) = server.run() {
        error!("Fail to run server: {:?}", e);
    };
    Ok(())
}
