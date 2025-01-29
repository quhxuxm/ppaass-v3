mod tcp;
mod udp;
use crate::ForwardProxyInfo;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use ppaass_common::{
    check_proxy_init_tunnel_response, parse_to_socket_addresses,
    receive_proxy_tunnel_init_response, send_proxy_tunnel_init_request, ProxyTcpConnection,
    UnifiedAddress,
};
use rand::random;
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
    ) -> Result<Self, CommonError> {
        let destination_tcp_connection =
            DestinationTcpEndpoint::connect(destination_address, keep_alive).await?;
        Ok(Self::Tcp(destination_tcp_connection))
    }

    pub async fn start_forward<T>(
        forward_proxies: &[ForwardProxyInfo],
        forward_rsa_crypto_repo: &T,
        destination_address: UnifiedAddress,
    ) -> Result<Self, CommonError>
    where
        T: RsaCryptoRepository + Send + Sync + 'static,
    {
        let proxy_index = random::<u32>() % forward_proxies.len() as u32;
        let proxy_info = &forward_proxies[proxy_index as usize];
        let mut proxy_tcp_connection = ProxyTcpConnection::create(
            proxy_info.proxy_auth.clone(),
            parse_to_socket_addresses(vec![&proxy_info.proxy_address])?.as_slice(),
            forward_rsa_crypto_repo,
        )
        .await?;

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
