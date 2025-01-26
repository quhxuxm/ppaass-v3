use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use ppaass_common::{ProxyTcpConnection, UnifiedAddress};
use std::net::SocketAddr;

use crate::config::AgentConfig;
use std::sync::Arc;
use tokio::net::TcpStream;
use tower::ServiceBuilder;
use tracing::debug;
async fn client_http_request_handler(
    client_http_request: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, CommonError> {
    let destination_uri = client_http_request.uri();
    let destination_host = destination_uri.host().ok_or(CommonError::Other(format!(
        "Can not find destination host: {destination_uri}"
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
    debug!("Receive client http request to destination: {destination_address:?}");
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
    let service_fn = ServiceBuilder::new().service(service_fn(client_http_request_handler));
    http1::Builder::new()
        .serve_connection(client_tcp_io, service_fn)
        .await?;
    Ok(())
}
