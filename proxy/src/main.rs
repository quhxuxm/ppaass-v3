mod command;
mod config;
mod error;

mod crypto;
mod tunnel;
use crate::command::Command;
use clap::Parser;
pub use config::*;
use ppaass_common::crypto::FileSystemRsaCryptoRepo;
use ppaass_common::server::{CommonServer, Server, ServerListener, ServerState};
use ppaass_common::{init_logger, ProxyTcpConnectionPool, ProxyTcpConnectionPoolConfig};

use crate::crypto::ForwardProxyRsaCryptoRepository;
use crate::tunnel::handle_agent_connection;
use std::fs::read_to_string;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use ppaass_common::error::CommonError;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Builder;
use tokio_tfo::TfoListener;
use tracing::{debug, error, trace};
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
const USER_AGENT_PUBLIC_KEY: &str = "AgentPublicKey.pem";
const USER_PROXY_PRIVATE_KEY: &str = "ProxyPrivateKey.pem";

const FORWARD_USER_AGENT_PRIVATE_KEY: &str = "AgentPrivateKey.pem";
const FORWARD_USER_PROXY_PUBLIC_KEY: &str = "ProxyPublicKey.pem";

const DEFAULT_CONFIG_FILE: &str = "resources/config.toml";

async fn create_server_listener(config: Arc<ProxyConfig>) -> Result<ServerListener, CommonError> {
    if config.ip_v6() {
        debug!(
            "Starting server listener with IPv6 on port: {}",
            config.server_port()
        );
        Ok(ServerListener::TfoListener(
            TfoListener::bind(SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                config.server_port(),
            ))
            .await?,
        ))
    } else {
        debug!(
            "Starting server listener with IPv4 on port: {}",
            config.server_port()
        );
        Ok(ServerListener::TfoListener(
            TfoListener::bind(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                config.server_port(),
            ))
            .await?,
        ))
    }
}

async fn start_server(
    config: Arc<ProxyConfig>,
    agent_rsa_crypto_repo: Arc<FileSystemRsaCryptoRepo>,
) -> Result<(), CommonError> {
    let mut server_state = ServerState::new();
    server_state.add_value(agent_rsa_crypto_repo.clone());
    if config.forward_proxies().is_some() {
        let forward_rsa_dir = config.forward_rsa_dir().as_ref().ok_or(CommonError::Other(
            "Fail to get forward rsa dir from configuration".to_string(),
        ))?;
        let forward_proxy_rsa_crypto_repo = Arc::new(ForwardProxyRsaCryptoRepository::new(
            FileSystemRsaCryptoRepo::new(
                forward_rsa_dir,
                FORWARD_USER_PROXY_PUBLIC_KEY,
                FORWARD_USER_AGENT_PRIVATE_KEY,
            )?,
        ));
        trace!(
            "Success to create forward proxy rsa crypto repo: {forward_proxy_rsa_crypto_repo:?}"
        );
        server_state.add_value(forward_proxy_rsa_crypto_repo.clone());
        if config.max_pool_size() > 1 {
            let proxy_tcp_connection_pool = ProxyTcpConnectionPool::new(
                config.clone(),
                forward_proxy_rsa_crypto_repo.clone(),
                config.clone(),
            )
            .await?;
            server_state.add_value(Arc::new(proxy_tcp_connection_pool));
        }
    }

    let server = CommonServer::new(config.clone(), server_state);
    server
        .run(create_server_listener, handle_agent_connection)
        .await?;
    Ok(())
}
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
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(config.worker_thread_number())
        .build()?;
    runtime.block_on(async move {
        if let Err(e) = start_server(config, rsa_crypto_repo).await {
            error!("Fail to start proxy server: {e:?}")
        }
    });
    Ok(())
}
