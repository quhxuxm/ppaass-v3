mod http;
mod socks4;
mod socks5;
use futures_util::{SinkExt, StreamExt};
pub use http::*;
use ppaass_common::error::CommonError;
use ppaass_common::{
    ProxyTcpConnection, TunnelInitFailureReason, TunnelInitRequest, TunnelInitResponse,
    UnifiedAddress,
};
pub use socks4::*;
pub use socks5::*;
use std::net::SocketAddr;
use tracing::debug;
async fn send_proxy_tunnel_init_request(
    proxy_tcp_connection: &mut ProxyTcpConnection,
    proxy_socket_address: SocketAddr,
    destination_address: UnifiedAddress,
) -> Result<(), CommonError> {
    let tunnel_init_request = TunnelInitRequest::Tcp {
        destination_address,
        keep_alive: false,
    };
    let tunnel_init_request_bytes = bincode::serialize(&tunnel_init_request)?;
    proxy_tcp_connection
        .send(&tunnel_init_request_bytes)
        .await?;
    debug!("Success to send tunnel init request to proxy: {proxy_socket_address}");
    Ok(())
}
async fn receive_proxy_tunnel_init_response(
    proxy_tcp_connection: &mut ProxyTcpConnection,
    proxy_socket_address: SocketAddr,
) -> Result<TunnelInitResponse, CommonError> {
    match proxy_tcp_connection.next().await {
        None => Err(CommonError::ConnectionExhausted(proxy_socket_address)),
        Some(Err(e)) => Err(e),
        Some(Ok(tunnel_init_response_bytes)) => {
            Ok(bincode::deserialize(&tunnel_init_response_bytes)?)
        }
    }
}
fn check_proxy_init_tunnel_response(
    tunnel_init_response: TunnelInitResponse,
) -> Result<(), CommonError> {
    match tunnel_init_response {
        TunnelInitResponse::Success => Ok(()),
        TunnelInitResponse::Failure(TunnelInitFailureReason::AuthenticateFail) => {
            Err(CommonError::Other(format!(
                "Tunnel init fail on authenticate: {tunnel_init_response:?}",
            )))
        }
        TunnelInitResponse::Failure(TunnelInitFailureReason::InitWithDestinationFail) => {
            Err(CommonError::Other(format!(
                "Tunnel init fail on connect destination: {tunnel_init_response:?}",
            )))
        }
    }
}
