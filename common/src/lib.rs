use futures_util::StreamExt;
pub mod config;
mod connection;
pub mod crypto;
pub mod error;
pub mod server;
use crate::error::CommonError;
pub use connection::*;
use futures_util::SinkExt;
pub use ppaass_protocol::*;
use rand::random;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::str::FromStr;
use tracing::{debug, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::time::ChronoUtc;
use uuid::Uuid;
/// Generate a random UUID
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string().replace("-", "").to_uppercase()
}

/// Generate a random 32 bytes vector
pub fn random_32_bytes() -> Vec<u8> {
    let random_32_bytes = random::<[u8; 32]>();
    random_32_bytes.to_vec()
}

/// Init the logger
pub fn init_logger(
    // The folder to store the log file
    log_folder: &Path,
    // The log name prefix
    log_name_prefix: &str,
    // The max log level
    max_log_level: &str,
) -> Result<WorkerGuard, CommonError> {
    let (trace_file_appender, _trace_appender_guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::daily(log_folder, log_name_prefix),
    );
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(max_log_level)?)
        .with_writer(trace_file_appender)
        .with_line_number(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_timer(ChronoUtc::rfc_3339())
        .with_ansi(false)
        .init();
    Ok(_trace_appender_guard)
}

pub fn parse_to_socket_addresses<I, T>(addresses: I) -> Result<Vec<SocketAddr>, CommonError>
where
    I: Iterator<Item = T>,
    T: AsRef<str>,
{
    let proxy_addresses = addresses
        .into_iter()
        .filter_map(|addr| addr.as_ref().to_socket_addrs().ok())
        .flatten()
        .collect::<Vec<SocketAddr>>();
    Ok(proxy_addresses)
}

pub async fn send_proxy_tunnel_init_request(
    proxy_tcp_connection: &mut ProxyTcpConnection,
    proxy_socket_address: SocketAddr,
    destination_address: UnifiedAddress,
) -> Result<(), CommonError> {
    let tunnel_init_request = TunnelControlRequest::TunnelInit(TunnelInitRequest::Tcp {
        destination_address,
        keep_alive: false,
    });
    let tunnel_init_request_bytes = bincode::serialize(&tunnel_init_request)?;
    proxy_tcp_connection
        .send(&tunnel_init_request_bytes)
        .await?;
    debug!("Success to send tunnel init request to proxy: {proxy_socket_address}");
    Ok(())
}
pub async fn receive_proxy_tunnel_init_response(
    proxy_tcp_connection: &mut ProxyTcpConnection,
    proxy_socket_address: SocketAddr,
) -> Result<TunnelInitResponse, CommonError> {
    loop {
        match proxy_tcp_connection.next().await {
            None => return Err(CommonError::ConnectionExhausted(proxy_socket_address)),
            Some(Err(e)) => return Err(e),
            Some(Ok(tunnel_init_response_bytes)) => {
                let tunnel_init_response: TunnelControlResponse =
                    bincode::deserialize(&tunnel_init_response_bytes)?;
                match tunnel_init_response {
                    TunnelControlResponse::Heartbeat(heartbeat_response) => {
                        debug!("Received heartbeat response: {heartbeat_response:?}");
                        continue;
                    }
                    TunnelControlResponse::TunnelInit(tunnel_init_response) => {
                        return Ok(tunnel_init_response)
                    }
                }
            }
        }
    }
}
pub fn check_proxy_init_tunnel_response(
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
