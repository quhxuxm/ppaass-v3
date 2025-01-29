mod client;

use crate::config::AgentConfig;
pub use client::*;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::{debug, error};

const SOCKS5_VERSION: u8 = 0x05;
const SOCKS4_VERSION: u8 = 0x04;
fn resolve_proxy_address(config: &AgentConfig) -> Result<Vec<SocketAddr>, CommonError> {
    let proxy_addresses = config
        .proxy_addresses()
        .iter()
        .filter_map(|addr| addr.to_socket_addrs().ok())
        .flatten()
        .collect::<Vec<SocketAddr>>();
    Ok(proxy_addresses)
}

pub async fn handle_client_connection<R>(
    config: Arc<AgentConfig>,
    rsa_crypto_repo: Arc<R>,
    client_tcp_stream: TcpStream,
    client_socket_address: SocketAddr,
) -> Result<(), CommonError>
where
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    let client_tcp_stream = client_tcp_stream;
    let client_socket_addr = client_socket_address;
    let mut protocol = [0u8; 1];
    let peek_size = client_tcp_stream.peek(&mut protocol).await?;
    if peek_size == 0 {
        error!("Client tcp stream exhausted: {client_socket_addr}");
        return Err(CommonError::ConnectionExhausted(client_socket_addr));
    }
    match protocol[0] {
        SOCKS5_VERSION => {
            debug!("Client tcp stream using socks5 protocol: {client_socket_addr}");
            socks5_protocol_proxy(
                client_tcp_stream,
                config,
                rsa_crypto_repo,
                client_socket_addr,
            )
            .await
        }
        SOCKS4_VERSION => {
            debug!("Client tcp stream using socks4 protocol: {client_socket_addr}");
            socks4_protocol_proxy(
                client_tcp_stream,
                config,
                rsa_crypto_repo,
                client_socket_addr,
            )
            .await
        }
        _ => {
            debug!("Client tcp stream using http protocol: {client_socket_addr}");
            http_protocol_proxy(
                client_tcp_stream,
                config,
                rsa_crypto_repo,
                client_socket_addr,
            )
            .await
        }
    }
}
