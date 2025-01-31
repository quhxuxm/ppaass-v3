use crate::command::Command;
use crate::config::AgentConfig;
use crate::tunnel::handle_client_connection;
use clap::Parser;
use ppaass_common::config::ServerConfig;
use ppaass_common::crypto::FileSystemRsaCryptoRepo;
use ppaass_common::error::CommonError;
use ppaass_common::server::{CommonServer, Server, ServerState};
use ppaass_common::{init_logger, ProxyTcpConnectionPool, ProxyTcpConnectionPoolConfig};
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Builder;
use tracing::error;
mod command;
mod config;
mod error;

mod tunnel;
const USER_SERVER_PUBLIC_KEY: &str = "ProxyPublicKey.pem";
const USER_AGENT_PRIVATE_KEY: &str = "AgentPrivateKey.pem";
const DEFAULT_CONFIG_FILE: &str = "resources/config.toml";

async fn start_server(
    config: Arc<AgentConfig>,
    rsa_crypto_repo: Arc<FileSystemRsaCryptoRepo>,
) -> Result<(), CommonError> {
    let mut server_state = ServerState::new();
    server_state.add_value(rsa_crypto_repo.clone());
    if let Some(_) = config.max_pool_size() {
        let proxy_tcp_connection_pool =
            ProxyTcpConnectionPool::new(config.clone(), rsa_crypto_repo.clone(), config.clone())
                .await?;
        server_state.add_value(Arc::new(proxy_tcp_connection_pool));
    }
    let server = CommonServer::new(config.clone(), server_state);
    server.run(handle_client_connection).await?;
    Ok(())
}

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

    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(config.worker_thread_number())
        .build()?;
    runtime.block_on(async move {
        if let Err(e) = start_server(config, rsa_crypto_repo).await {
            error!("Fail to start agent server: {e:?}")
        }
    });

    Ok(())
}
