mod tcp;
mod udp;
use crate::config::ForwardConfig;
use ppaass_common::config::ProxyTcpConnectionConfig;
use ppaass_common::error::CommonError;
use ppaass_common::server::ServerState;
use ppaass_common::user::UserInfo;
use ppaass_common::{
    ProxyTcpConnection, ProxyTcpConnectionPool, ProxyTcpConnectionRelayState, TunnelInitRequest,
    UnifiedAddress,
};
use std::sync::Arc;
pub use tcp::*;
use tokio::sync::RwLock;
use tracing::debug;
pub use udp::*;
pub enum DestinationEdge {
    Tcp(DestinationTcpEndpoint),
    Forward(ProxyTcpConnection<ProxyTcpConnectionRelayState>),
    Udp(DestinationUdpEndpoint),
}

impl DestinationEdge {
    pub async fn start_tcp(
        destination_address: UnifiedAddress,
        keep_alive: bool,
        connect_timeout: u64,
    ) -> Result<Self, CommonError> {
        let destination_tcp_connection =
            DestinationTcpEndpoint::connect(destination_address, keep_alive, connect_timeout)
                .await?;
        Ok(Self::Tcp(destination_tcp_connection))
    }

    pub async fn start_forward(
        server_state: &ServerState,
        forward_config: &ForwardConfig,
        destination_address: UnifiedAddress,
    ) -> Result<Self, CommonError> {
        let (username, user_info) = server_state
            .get_value::<(String, Arc<RwLock<UserInfo>>)>()
            .ok_or(CommonError::Other("Can not find forward user".to_owned()))?;
        let proxy_tcp_connection_pool =
            match server_state.get_value::<Arc<ProxyTcpConnectionPool<ForwardConfig>>>() {
                None => {
                    let user_info = user_info.read().await;
                    ProxyTcpConnection::create(
                        &username,
                        &user_info,
                        forward_config.proxy_frame_size(),
                        forward_config.proxy_connect_timeout(),
                    )
                    .await?
                }
                Some(pool) => pool.take_proxy_connection().await?,
            };
        let proxy_socket_address = proxy_tcp_connection_pool.proxy_socket_address();
        debug!("Success to create forward proxy tcp connection: {proxy_socket_address}");
        let proxy_tcp_connection = proxy_tcp_connection_pool
            .tunnel_init(TunnelInitRequest::Tcp {
                destination_address: destination_address.clone(),
                keep_alive: false,
            })
            .await?;
        debug!(
            "Success to send init tunnel request on forward proxy tcp connection: {proxy_socket_address}, destination address: {destination_address:?}"
        );
        Ok(Self::Forward(proxy_tcp_connection))
    }

    pub async fn start_udp() -> Result<Self, CommonError> {
        Ok(Self::Udp(DestinationUdpEndpoint::new()))
    }
}
