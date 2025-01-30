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

use tracing::{debug, error, trace};
const USER_AGENT_PUBLIC_KEY: &str = "AgentPublicKey.pem";
const USER_PROXY_PRIVATE_KEY: &str = "ProxyPrivateKey.pem";

const FORWARD_USER_AGENT_PRIVATE_KEY: &str = "AgentPrivateKey.pem";
const FORWARD_USER_PROXY_PUBLIC_KEY: &str = "ProxyPublicKey.pem";

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
    let rsa_dir = command
        .agent_rsa_dir
        .unwrap_or(config.agent_rsa_dir().clone());
    debug!("Rsa directory of the proxy server: {rsa_dir:?}");
    let rsa_crypto_repo = Arc::new(FileSystemRsaCryptoRepo::new(
        &rsa_dir,
        USER_AGENT_PUBLIC_KEY,
        USER_PROXY_PRIVATE_KEY,
    )?);
    trace!("Success to create agent_rsa crypto repo: {rsa_crypto_repo:?}");
    let forward_rsa_dir = match command.forward_rsa {
        None => match config.forward_rsa_dir() {
            None => None,
            Some(forward_rsa_dir) => Some(forward_rsa_dir.clone()),
        },
        Some(forward_rsa_dir) => Some(forward_rsa_dir),
    };
    debug!("Forward agent_rsa directory of the proxy server: {forward_rsa_dir:?}");
    let forward_rsa_crypto_repo = match forward_rsa_dir {
        None => None,
        Some(forward_rsa_dir) => Some(Arc::new(FileSystemRsaCryptoRepo::new(
            &forward_rsa_dir,
            FORWARD_USER_PROXY_PUBLIC_KEY,
            FORWARD_USER_AGENT_PRIVATE_KEY,
        )?)),
    };
    trace!("Success to create forward agent_rsa crypto repo: {rsa_crypto_repo:?}");
    let server = CommonServer::new(
        config.clone(),
        config,
        rsa_crypto_repo,
        forward_rsa_crypto_repo.clone(),
    );
    if let Err(e) = server.run(
        move |config,
              rsa_crypto_repo,
              agent_tcp_stream,
              agent_socket_address,
              forward_proxy_tcp_connection_pool| {
            let forward_rsa_crypto_repo = forward_rsa_crypto_repo.clone();
            handle_agent_connection(
                config,
                rsa_crypto_repo,
                forward_rsa_crypto_repo,
                agent_tcp_stream,
                agent_socket_address,
                forward_proxy_tcp_connection_pool,
            )
        },
    ) {
        error!("Fail to run proxy: {:?}", e);
    };
    Ok(())
}
