use crate::config::AgentConfig;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1::Builder;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use ppaass_common::error::CommonError;
use ppaass_common::{
    ProxyTcpConnection, ProxyTcpConnectionPool, TunnelInitRequest, UnifiedAddress,
};

use ppaass_common::server::ServerState;

use ppaass_common::config::ProxyTcpConnectionConfig;
use ppaass_common::user::UserInfo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tfo::TfoStream;
use tower::ServiceBuilder;
use tracing::{debug, error, info};
fn success_empty_body() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

async fn client_http_request_handler(
    config: &AgentConfig,
    username: &str,
    user_info: &UserInfo,
    server_state: Arc<ServerState>,
    client_socket_addr: SocketAddr,
    client_http_request: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, CommonError> {
    let destination_uri = client_http_request.uri();
    let destination_host = destination_uri.host().ok_or(CommonError::Other(format!(
        "Can not find destination host: {destination_uri}, client socket address: {client_socket_addr}"
    )))?;
    let destination_port = destination_uri.port().map(|port| port.as_u16());
    let destination_address = if client_http_request.method() == Method::CONNECT {
        UnifiedAddress::Domain {
            host: destination_host.to_string(),
            port: destination_port.unwrap_or(443),
        }
    } else {
        UnifiedAddress::Domain {
            host: destination_host.to_string(),
            port: destination_port.unwrap_or(80),
        }
    };
    debug!(
        "Receive client http request to destination: {destination_address:?}, client socket address: {client_socket_addr}"
    );
    let proxy_tcp_connection_pool =
        server_state.get_value::<Arc<ProxyTcpConnectionPool<AgentConfig>>>();
    let proxy_tcp_connection = match proxy_tcp_connection_pool {
        None => {
            ProxyTcpConnection::create(
                username,
                user_info,
                config.proxy_frame_size(),
                config.proxy_connect_timeout(),
            )
            .await?
        }
        Some(pool) => pool.take_proxy_connection().await?,
    };
    let proxy_socket_address = proxy_tcp_connection.proxy_socket_address();
    debug!("Going to initialize tunnel with proxy: {proxy_socket_address}");
    let mut proxy_tcp_connection = proxy_tcp_connection
        .tunnel_init(TunnelInitRequest::Tcp {
            destination_address,
            keep_alive: false,
        })
        .await?;

    if Method::CONNECT == client_http_request.method() {
        // Received an HTTP request like:
        // ```
        // CONNECT www.domain.com:443 HTTP/1.1
        // Host: www.domain.com:443
        // Proxy-Connection: Keep-Alive
        // ```
        //
        // When HTTP method is CONNECT we should return an empty body
        // then we can eventually upgrade the connection and talk a new protocol.
        //
        // Note: only after client received an empty body with STATUS_OK can the
        // connection be upgraded, so we can't return a response inside
        // `on_upgrade` future.
        let agent_to_proxy_data_relay_buffer_size = config.agent_to_proxy_data_relay_buffer_size();
        let proxy_to_agent_data_relay_buffer_size = config.proxy_to_agent_data_relay_buffer_size();
        tokio::task::spawn(async move {
            match hyper::upgrade::on(client_http_request).await {
                Err(e) => {
                    error!("Failed to upgrade client http request: {e}");
                    return;
                }
                Ok(upgraded_client_io) => {
                    // Connect to remote server
                    let mut upgraded_client_io = TokioIo::new(upgraded_client_io);
                    // Proxying data
                    let (from_client, from_proxy) = match tokio::io::copy_bidirectional_with_sizes(
                        &mut upgraded_client_io,
                        &mut proxy_tcp_connection,
                        agent_to_proxy_data_relay_buffer_size,
                        proxy_to_agent_data_relay_buffer_size,
                    )
                    .await
                    {
                        Err(e) => {
                            error!("Fail to proxy data between agent and proxy: {e:?}");
                            return;
                        }
                        Ok((from_client, from_proxy)) => (from_client, from_proxy),
                    };

                    // Print message when done
                    info!(
                        "Agent wrote {} bytes to proxy, received {} bytes from proxy",
                        from_client, from_proxy
                    );
                }
            }
        });
        Ok(Response::new(success_empty_body()))
    } else {
        let proxy_tcp_connection = TokioIo::new(proxy_tcp_connection);
        let (mut proxy_tcp_connection_sender, proxy_tcp_connection_obj) = Builder::new()
            .preserve_header_case(true)
            .title_case_headers(true)
            .handshake(proxy_tcp_connection)
            .await?;
        tokio::spawn(async move {
            if let Err(err) = proxy_tcp_connection_obj.await {
                error!("Proxy tcp connection failed: {:?}", err);
            }
        });

        let proxy_response = proxy_tcp_connection_sender
            .send_request(client_http_request)
            .await?;
        Ok(proxy_response.map(|b| b.boxed()))
    }
}

pub async fn http_protocol_proxy(
    client_tcp_stream: TfoStream,
    client_socket_addr: SocketAddr,
    config: &AgentConfig,
    username: &str,
    user_info: Arc<RwLock<UserInfo>>,
    server_state: Arc<ServerState>,
) -> Result<(), CommonError> {
    let client_tcp_io = TokioIo::new(client_tcp_stream);
    let service_fn = ServiceBuilder::new().service(service_fn(|request| {
        let server_state = server_state.clone();
        async {
            let user_info = user_info.read().await;
            client_http_request_handler(
                config,
                username,
                &user_info,
                server_state,
                client_socket_addr,
                request,
            )
            .await
        }
    }));
    http1::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .serve_connection(client_tcp_io, service_fn)
        .with_upgrades()
        .await?;
    Ok(())
}
