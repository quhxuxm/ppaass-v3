mod tcp;
mod udp;
use crate::crypto::ForwardProxyRsaCryptoRepository;
use crate::ProxyConfig;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use ppaass_common::server::ServerState;
use ppaass_common::{
    check_proxy_init_tunnel_response, receive_proxy_tunnel_init_response,
    send_proxy_tunnel_init_request, ProxyTcpConnection, ProxyTcpConnectionInfo,
    ProxyTcpConnectionPool, UnifiedAddress,
};
use std::sync::Arc;
pub use tcp::*;
use tracing::debug;
pub use udp::*;
pub enum DestinationEdge {
    Tcp(DestinationTcpEndpoint),
    Forward(ProxyTcpConnection),
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
        let mut proxy_tcp_connection =
            match server_state.get_value::<Arc<
                ProxyTcpConnectionPool<ProxyConfig, ProxyConfig, ForwardProxyRsaCryptoRepository>,
            >>() {
                None => {
                    ProxyTcpConnection::create(proxy_tcp_connection_info, forward_rsa_crypto_repo)
                        .await?
                }
                Some(pool) => pool.take_proxy_connection().await?,
            };
        let proxy_socket_address = proxy_tcp_connection.proxy_socket_address();
        debug!("Success to create forward proxy tcp connection: {proxy_socket_address}");
        send_proxy_tunnel_init_request(
            &mut proxy_tcp_connection,
            proxy_socket_address,
            destination_address.clone(),
        )
        .await?;
        debug!("Success to send init tunnel request on forward proxy tcp connection: {proxy_socket_address}, destination address: {destination_address:?}");
        let tunnel_init_response =
            receive_proxy_tunnel_init_response(&mut proxy_tcp_connection, proxy_socket_address)
                .await?;
        debug!("Success to receive init tunnel response on forward proxy tcp connection: {proxy_socket_address}, response: {tunnel_init_response:?}");
        check_proxy_init_tunnel_response(tunnel_init_response)?;
        Ok(Self::Forward(proxy_tcp_connection))
    }

    pub async fn start_udp() -> Result<Self, CommonError> {
        Ok(Self::Udp(DestinationUdpEndpoint::new()))
    }
}
