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
use ppaass_common::user::UserInfoRepository;
use ppaass_common::ProxyTcpConnectionPool;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;
use tracing::{debug, info};
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
    let (username, user_info) = loop {
        match user_repo.get_single_user().await? {
            None => {
                sleep(Duration::from_secs(5)).await;
                continue;
            }
            Some(element) => break element,
        }
    };
    info!("Start agent server with username: {}", &username);
    server_state.add_value((username.clone(), user_info.clone()));
    if config.connection_pool().is_some() {
        let proxy_tcp_connection_pool =
            ProxyTcpConnectionPool::new(config.clone(), &username, user_info.clone()).await?;
        server_state.add_value(Arc::new(proxy_tcp_connection_pool));
    }
    let server = CommonServer::new(config.clone(), server_state);
    server
        .run(create_server_listener, handle_client_connection)
        .await?;
    Ok(())
}
