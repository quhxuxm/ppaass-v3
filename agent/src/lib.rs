mod command;
mod config;
mod error;
mod tunnel;

pub use command::Command;
pub use config::AgentConfig;
use ppaass_common::config::ServerConfig;
use ppaass_common::error::CommonError;
use ppaass_common::server::{CommonServer, Server, ServerListener, ServerState};
use ppaass_common::user::repo::fs::FileSystemUserInfoRepository;
use ppaass_common::ProxyTcpConnectionPool;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::debug;
pub use tunnel::handle_client_connection;

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

pub async fn start_server(
    config: Arc<AgentConfig>,
    user_repo: Arc<FileSystemUserInfoRepository>,
) -> Result<(), CommonError> {
    let mut server_state = ServerState::new();
    server_state.add_value(user_repo.clone());
    if let Some(connection_pool_config) = config.connection_pool() {
        let proxy_tcp_connection_pool = ProxyTcpConnectionPool::new(
            Arc::new(connection_pool_config.clone()),
            user_repo.clone(),
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
