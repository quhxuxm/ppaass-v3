use crate::config::AgentConfig;
use ppaass_common::config::ProxyTcpConnectionConfig;
use ppaass_common::error::CommonError;
use ppaass_common::server::ServerState;
use ppaass_common::user::UserInfo;
use ppaass_common::{
    FramedConnection, ProxyTcpConnectionNewState, ProxyTcpConnectionPool, TunnelInitRequest,
    UnifiedAddress,
};
use socks5_impl::protocol::handshake::Request as Socks5HandshakeRequest;
use socks5_impl::protocol::handshake::Response as Socks5HandshakeResponse;
use socks5_impl::protocol::{Address, AsyncStreamOperation, AuthMethod, Reply};
use socks5_impl::protocol::{
    Command as Socks5InitCommand, Request as Socks5InitRequest, Response as Socks5InitResponse,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tfo::TfoStream;
use tracing::{debug, error, info};
pub async fn socks5_protocol_proxy(
    mut client_tcp_stream: TfoStream,
    client_socket_addr: SocketAddr,
    config: &AgentConfig,
    username: &str,
    user_info: Arc<RwLock<UserInfo>>,
    server_state: Arc<ServerState>,
) -> Result<(), CommonError> {
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
        Socks5InitCommand::Connect => {
            debug!("Receive socks5 CONNECT command: {client_socket_addr}");
            let proxy_tcp_connection_pool =
                server_state.get_value::<Arc<ProxyTcpConnectionPool<AgentConfig>>>();
            let proxy_tcp_connection = match proxy_tcp_connection_pool {
                None => {
                    let user_info = user_info.read().await;
                    FramedConnection::<ProxyTcpConnectionNewState>::create(
                        username,
                        &user_info,
                        config.proxy_frame_size(),
                        config.proxy_connect_timeout(),
                    )
                    .await?
                }
                Some(pool) => pool.take_proxy_connection().await?,
            };

            let destination_address = match &init_request.address {
                Address::SocketAddress(dst_addr) => dst_addr.into(),
                Address::DomainAddress(host, port) => UnifiedAddress::Domain {
                    host: host.clone(),
                    port: *port,
                },
            };

            let mut proxy_tcp_connection = proxy_tcp_connection
                .tunnel_init(TunnelInitRequest::Tcp {
                    destination_address,
                    keep_alive: false,
                })
                .await?;

            let init_response = Socks5InitResponse::new(Reply::Succeeded, init_request.address);
            init_response
                .write_to_async_stream(&mut client_tcp_stream)
                .await?;

            // Proxying data
            let (from_client, from_proxy) = match tokio::io::copy_bidirectional_with_sizes(
                &mut client_tcp_stream,
                &mut proxy_tcp_connection,
                config.agent_to_proxy_data_relay_buffer_size(),
                config.proxy_to_agent_data_relay_buffer_size(),
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
        Socks5InitCommand::Bind => {
            debug!("Receive socks5 BIND command: {client_socket_addr:?}");
            return Err(CommonError::Other(format!(
                "Unsupported socks5 bind command: {client_socket_addr}"
            )));
        }
        Socks5InitCommand::UdpAssociate => {
            debug!("Receive socks5 UDP ASSOCIATE command: {client_socket_addr:?}");
            return Err(CommonError::Other(format!(
                "Unsupported socks5 udp associate command: {client_socket_addr}"
            )));
        }
    }
    Ok(())
}
