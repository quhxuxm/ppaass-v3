mod tcp;
mod udp;
use crate::crypto::ForwardProxyRsaCryptoRepository;
use crate::ForwardConfig;
use ppaass_common::config::ConnectionPoolConfig;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use ppaass_common::server::ServerState;
use ppaass_common::{
    ProxyTcpConnection, ProxyTcpConnectionInfo, ProxyTcpConnectionPool,
    ProxyTcpConnectionRelayState, TunnelInitRequest, UnifiedAddress,
};
use std::sync::Arc;
pub use tcp::*;
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

    pub async fn start_forward<T>(
        server_state: &ServerState,
        proxy_tcp_connection_info: ProxyTcpConnectionInfo,
        forward_rsa_crypto_repo: &T,
        destination_address: UnifiedAddress,
    ) -> Result<Self, CommonError>
    where
        T: RsaCryptoRepository + Send + Sync + 'static,
    {
        let proxy_tcp_connection = match server_state.get_value::<Arc<
            ProxyTcpConnectionPool<
                ConnectionPoolConfig,
                ForwardConfig,
                ForwardProxyRsaCryptoRepository,
            >,
        >>() {
            None => {
                ProxyTcpConnection::create(proxy_tcp_connection_info, forward_rsa_crypto_repo)
                    .await?
            }
            Some(pool) => pool.take_proxy_connection().await?,
        };
        let proxy_socket_address = proxy_tcp_connection.proxy_socket_address();
        debug!("Success to create forward proxy tcp connection: {proxy_socket_address}");
        let proxy_tcp_connection = proxy_tcp_connection
            .tunnel_init(TunnelInitRequest::Tcp {
                destination_address: destination_address.clone(),
                keep_alive: false,
            })
            .await?;
        debug!("Success to send init tunnel request on forward proxy tcp connection: {proxy_socket_address}, destination address: {destination_address:?}");
        Ok(Self::Forward(proxy_tcp_connection))
    }

    pub async fn start_udp() -> Result<Self, CommonError> {
        Ok(Self::Udp(DestinationUdpEndpoint::new()))
    }
}
