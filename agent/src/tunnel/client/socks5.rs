use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::tunnel::resolve_proxy_address;
use futures_util::{SinkExt, StreamExt};
use hyper_util::rt::TokioIo;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use ppaass_common::{
    ProxyTcpConnection, TunnelInitFailureReason, TunnelInitRequest, TunnelInitResponse,
    UnifiedAddress,
};
use socks5_impl::protocol::handshake::Request as Socks5HandshakeRequest;
use socks5_impl::protocol::handshake::Response as Socks5HandshakeResponse;
use socks5_impl::protocol::{Address, AsyncStreamOperation, AuthMethod, Reply};
use socks5_impl::protocol::{
    Command, Request as Socks5InitRequest, Response as Socks5InitResponse,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::io::{SinkWriter, StreamReader};
use tracing::{debug, error, info};
pub async fn socks5_protocol_proxy<R>(
    mut client_tcp_stream: TcpStream,
    config: Arc<AgentConfig>,
    rsa_crypto_repo: Arc<R>,
    client_socket_addr: SocketAddr,
) -> Result<(), CommonError>
where
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    debug!("Client connect to agent with socks 5 protocol: {client_socket_addr}");
    let auth_request =
        Socks5HandshakeRequest::retrieve_from_async_stream(&mut client_tcp_stream).await?;
    debug!("Receive client socks5 handshake auth request: {auth_request:?}");
    let auth_response = Socks5HandshakeResponse::new(AuthMethod::NoAuth);
    auth_response
        .write_to_async_stream(&mut client_tcp_stream)
        .await?;
    let init_request =
        Socks5InitRequest::retrieve_from_async_stream(&mut client_tcp_stream).await?;
    debug!("Receive client socks5 handshake init request: {init_request:?}");
    match init_request.command {
        Command::Connect => {
            debug!("Receive socks5 CONNECT command: {client_tcp_stream:?}");
            let mut proxy_tcp_connection = ProxyTcpConnection::create(
                config.authentication().to_owned(),
                resolve_proxy_address(&config)?.as_slice(),
                rsa_crypto_repo.as_ref(),
            )
            .await?;

            let proxy_socket_address = proxy_tcp_connection.proxy_socket_address();
            let destination_address = match &init_request.address {
                Address::SocketAddress(dst_addr) => dst_addr.into(),
                Address::DomainAddress(host, port) => UnifiedAddress::Domain {
                    host: host.clone(),
                    port: *port,
                },
            };
            let tunnel_init_request = TunnelInitRequest::Tcp {
                destination_address: destination_address.clone(),
                keep_alive: false,
            };
            let tunnel_init_request_bytes = bincode::serialize(&tunnel_init_request)?;
            proxy_tcp_connection
                .send(&tunnel_init_request_bytes)
                .await?;
            let tunnel_init_response = match proxy_tcp_connection.next().await {
                None => return Err(CommonError::ConnectionExhausted(proxy_socket_address)),
                Some(Err(e)) => return Err(e),
                Some(Ok(tunnel_init_response_bytes)) => {
                    bincode::deserialize(&tunnel_init_response_bytes)?
                }
            };
            match tunnel_init_response {
                TunnelInitResponse::Success => {
                    debug!("Tunnel init success: {destination_address:?}, client socket address: {client_socket_addr}");
                }
                TunnelInitResponse::Failure(TunnelInitFailureReason::AuthenticateFail) => {
                    return Err(CommonError::Other(format!(
                        "Tunnel init fail because of authentication: {}, client socket address: {client_socket_addr}",
                        config.authentication()
                    )))
                }
                TunnelInitResponse::Failure(TunnelInitFailureReason::InitWithDestinationFail) => {
                    return Err(CommonError::Other(format!(
                        "Tunnel init fail because of destination connect fail: {destination_address}, client socket address: {client_socket_addr}"
                    )))
                }
            }
            debug!("Socks5 client tunnel init success with remote: {proxy_socket_address:?}");
            let init_response = Socks5InitResponse::new(Reply::Succeeded, init_request.address);
            init_response
                .write_to_async_stream(&mut client_tcp_stream)
                .await?;
            debug!("Socks5 client tunnel init success begin to relay, : {proxy_socket_address:?}");

            let proxy_tcp_connection = StreamReader::new(proxy_tcp_connection);
            let mut proxy_tcp_connection = SinkWriter::new(proxy_tcp_connection);

            // Proxying data
            let (from_client, from_proxy) = match tokio::io::copy_bidirectional(
                &mut client_tcp_stream,
                &mut proxy_tcp_connection,
            )
            .await
            {
                Err(e) => {
                    error!("Fail to proxy data between agent and proxy: {e:?}");
                    return Ok(());
                }
                Ok((from_client, from_proxy)) => (from_client, from_proxy),
            };
            info!(
                "Agent wrote {} bytes to proxy, received {} bytes from proxy",
                from_client, from_proxy
            );
        }
        Command::Bind => {
            debug!("Receive socks5 BIND command: {client_tcp_stream:?}");
            return Err(CommonError::Other(format!(
                "Unsupported socks5 bind command: {client_socket_addr}"
            )));
        }
        Command::UdpAssociate => {
            debug!("Receive socks5 UDP ASSOCIATE command: {client_tcp_stream:?}");
            return Err(CommonError::Other(format!(
                "Unsupported socks5 udp associate command: {client_socket_addr}"
            )));
        }
    }
    Ok(())
}
