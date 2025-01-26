use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use ppaass_common::error::CommonError;
use ppaass_protocol::UnifiedAddress;
use std::convert::Infallible;
use tokio::net::TcpStream;
use tracing::debug;
pub struct ClientHttpConnection {
    client_tcp_stream: TcpStream,
}

impl ClientHttpConnection {
    pub fn new(client_tcp_stream: TcpStream) -> Self {
        Self { client_tcp_stream }
    }

    async fn handle_client_http_request(
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
    pub async fn exec(self) -> Result<(), CommonError> {
        let client_tcp_io = TokioIo::new(self.client_tcp_stream);
        http1::Builder::new()
            .serve_connection(client_tcp_io, service_fn(Self::handle_client_http_request))
            .await?;
        Ok(())
    }
}
