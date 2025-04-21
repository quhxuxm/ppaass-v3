mod command;
mod config;
mod error;

mod tunnel;
mod user;
use crate::command::Command;
use clap::Parser;
pub use config::*;
use ppaass_common::server::{CommonServer, Server, ServerListener, ServerState};
use ppaass_common::{init_logger, ProxyTcpConnectionPool};

use crate::tunnel::handle_agent_connection;
use crate::user::ForwardProxyUserRepository;
use std::fs::read_to_string;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use ppaass_common::error::CommonError;
use ppaass_common::user::repo::fs::FileSystemUserInfoRepository;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tokio_tfo::TfoListener;
use tracing::{debug, error, trace};
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const DEFAULT_CONFIG_FILE: &str = "resources/config.toml";

async fn create_server_listener(config: Arc<ProxyConfig>) -> Result<ServerListener, CommonError> {
    if config.ip_v6() {
        debug!(
            "Starting server listener with IPv6 on port: {}",
            config.server_port()
        );
        Ok(ServerListener::TcpListener(
            TcpListener::bind(SocketAddr::new(
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
        Ok(ServerListener::TcpListener(
            TcpListener::bind(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                config.server_port(),
            ))
            .await?,
        ))
    }
}

async fn start_server(
    config: Arc<ProxyConfig>,
    agent_user_repo: Arc<FileSystemUserInfoRepository>,
) -> Result<(), CommonError> {
    let mut server_state = ServerState::new();
    server_state.add_value(agent_user_repo.clone());
    if let Some(forward_config) = config.forward() {
        let forward_proxy_user_repo = Arc::new(ForwardProxyUserRepository::new(
            FileSystemUserInfoRepository::new(forward_config.user_dir())?,
        ));
        trace!("Success to create forward proxy user crypto repo: {forward_proxy_user_repo:?}");
        server_state.add_value(forward_proxy_user_repo.clone());
        if let Some(connection_pool_config) = forward_config.connection_pool() {
            let proxy_tcp_connection_pool = ProxyTcpConnectionPool::new(
                Arc::new(connection_pool_config.clone()),
                forward_proxy_user_repo.clone(),
                Arc::new(forward_config.clone()),
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
    let user_dir = command
        .agent_rsa_dir
        .unwrap_or(config.user_dir().to_owned());
    debug!("Rsa directory of the proxy server: {user_dir:?}");
    let rsa_crypto_repo = Arc::new(FileSystemUserInfoRepository::new(&user_dir)?);
    trace!("Success to create agent_user crypto repo: {rsa_crypto_repo:?}");
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
