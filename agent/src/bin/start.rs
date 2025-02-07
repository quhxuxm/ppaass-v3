use clap::Parser;
use ppaass_agent::handle_client_connection;
use ppaass_agent::AgentConfig;
use ppaass_agent::Command;
use ppaass_common::config::ServerConfig;
use ppaass_common::crypto::FileSystemRsaCryptoRepo;
use ppaass_common::error::CommonError;
use ppaass_common::server::{CommonServer, Server, ServerListener, ServerState};
use ppaass_common::{init_logger, ProxyTcpConnectionPool};
use std::fs::read_to_string;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tracing::{debug, error};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
const USER_SERVER_PUBLIC_KEY: &str = "ProxyPublicKey.pem";
const USER_AGENT_PRIVATE_KEY: &str = "AgentPrivateKey.pem";
const DEFAULT_CONFIG_FILE: &str = "resources/config.toml";

async fn create_server_listener(config: Arc<AgentConfig>) -> Result<ServerListener, CommonError> {
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
    config: Arc<AgentConfig>,
    rsa_crypto_repo: Arc<FileSystemRsaCryptoRepo>,
) -> Result<(), CommonError> {
    let mut server_state = ServerState::new();
    server_state.add_value(rsa_crypto_repo.clone());
    if let Some(connection_pool_config) = config.connection_pool() {
        let proxy_tcp_connection_pool = ProxyTcpConnectionPool::new(
            Arc::new(connection_pool_config.clone()),
            rsa_crypto_repo.clone(),
            config.clone(),
        )
        .await?;
        server_state.add_value(Arc::new(proxy_tcp_connection_pool));
    }
    let server = CommonServer::new(config.clone(), server_state);
    server
        .run(create_server_listener, handle_client_connection)
        .await?;
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
