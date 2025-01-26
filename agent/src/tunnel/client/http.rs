use crate::config::AgentConfig;
use crate::tunnel::resolve_proxy_address;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use ppaass_common::{
    ProxyTcpConnection, TunnelInitFailureReason, TunnelInitRequest, TunnelInitResponse,
    UnifiedAddress,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::bytes::BytesMut;
use tower::ServiceBuilder;
use tracing::debug;
async fn client_http_request_handler<R>(
    config: &AgentConfig,
    rsa_crypto_repo: &R,
    client_socket_addr: SocketAddr,
    client_http_request: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, CommonError>
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
        .send(BytesMut::from_iter(tunnel_init_request_bytes))
        .await?;
    let tunnel_init_response = match proxy_tcp_connection.next().await {
        None => return Err(CommonError::ConnectionExhausted(proxy_socket_address)),
        Some(Err(e)) => return Err(e),
        Some(Ok(tunnel_init_response_bytes)) => bincode::deserialize(&tunnel_init_response_bytes)?,
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

    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
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
    let service_fn = ServiceBuilder::new().service(service_fn(|request| {
        // let config = config.clone();
        async {
            debug!("Begin to handle");
            client_http_request_handler(
                config.as_ref(),
                rsa_crypto_repo.as_ref(),
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
