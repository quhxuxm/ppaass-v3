mod config;
mod error;
mod tunnel;
pub use config::AgentConfig;
use ppaass_common::config::RetrieveServerConfig;
use ppaass_common::error::CommonError;
use ppaass_common::server::{CommonServer, Server, ServerListener, ServerState};
use ppaass_common::user::UserInfoRepository;
use ppaass_common::ProxyTcpConnectionPool;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, info};
use tunnel::handle_client_connection;
async fn create_server_listener<T: RetrieveServerConfig>(
    config: Arc<T>,
) -> Result<ServerListener, CommonError> {
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

pub async fn start_server<T>(config: Arc<AgentConfig>, user_repo: Arc<T>) -> Result<(), CommonError>
where
    T: UserInfoRepository + Send + Sync + 'static,
{
    let mut server_state = ServerState::new();
    let (username, user_info) = {
        let username = &config.username;
        let user_info = user_repo
            .get_user(username)
            .await?
            .ok_or(CommonError::Other(format!(
                "Can not get user info from repository: {username}"
            )))?;
        (&config.username.to_owned(), user_info)
    };
    info!("Start agent server with username: {}", &username);
    server_state.add_value((username.clone(), user_info.clone()));
    if config.connection_pool.is_some() {
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
