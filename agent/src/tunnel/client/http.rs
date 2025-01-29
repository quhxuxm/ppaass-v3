use crate::config::AgentConfig;
use crate::tunnel::resolve_proxy_address;
use futures_util::{SinkExt, StreamExt};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1::Builder;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use ppaass_common::{ProxyTcpConnection, TunnelInitRequest, UnifiedAddress};

use crate::tunnel::client::check_proxy_init_tunnel_response;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::io::{SinkWriter, StreamReader};
use tower::ServiceBuilder;
use tracing::{debug, error, info};
fn empty_body() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

async fn client_http_request_handler<R>(
    config: &AgentConfig,
    rsa_crypto_repo: &R,
    client_socket_addr: SocketAddr,
    client_http_request: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, CommonError>
where
    R: RsaCryptoRepository + Send + Sync + 'static,
{
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
    debug!("Receive client http request to destination: {destination_address:?}, client socket address: {client_socket_addr}");
    let mut proxy_tcp_connection = ProxyTcpConnection::create(
        config.authentication().to_owned(),
        resolve_proxy_address(config)?.as_slice(),
        rsa_crypto_repo,
    )
    .await?;
    let proxy_socket_address = proxy_tcp_connection.proxy_socket_address();
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
        Some(Ok(tunnel_init_response_bytes)) => bincode::deserialize(&tunnel_init_response_bytes)?,
    };
    check_proxy_init_tunnel_response(tunnel_init_response)?;
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
        tokio::task::spawn(async move {
            match hyper::upgrade::on(client_http_request).await {
                Err(e) => {
                    error!("Failed to upgrade client http request: {e}");
                    return;
                }
                Ok(upgraded_client_io) => {
                    // Connect to remote server
                    let proxy_tcp_connection = StreamReader::new(proxy_tcp_connection);
                    let mut proxy_tcp_connection = SinkWriter::new(proxy_tcp_connection);
                    let mut upgraded_client_io = TokioIo::new(upgraded_client_io);

                    // Proxying data
                    let (from_client, from_proxy) = match tokio::io::copy_bidirectional(
                        &mut upgraded_client_io,
                        &mut proxy_tcp_connection,
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
        Ok(Response::new(empty_body()))
    } else {
        let proxy_tcp_connection = StreamReader::new(proxy_tcp_connection);
        let proxy_tcp_connection = SinkWriter::new(proxy_tcp_connection);
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

pub async fn http_protocol_proxy<R>(
    client_tcp_stream: TcpStream,
    config: Arc<AgentConfig>,
    rsa_crypto_repo: Arc<R>,
    client_socket_addr: SocketAddr,
) -> Result<(), CommonError>
where
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    let client_tcp_io = TokioIo::new(client_tcp_stream);
    let service_fn = ServiceBuilder::new().service(service_fn(|request| async {
        client_http_request_handler(
            config.as_ref(),
            rsa_crypto_repo.as_ref(),
            client_socket_addr,
            request,
        )
        .await
    }));
    http1::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .serve_connection(client_tcp_io, service_fn)
        .with_upgrades()
        .await?;
    Ok(())
}
