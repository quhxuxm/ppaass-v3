mod client;

use crate::config::AgentConfig;
pub use client::*;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::debug;
pub struct Tunnel<R>
where
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    config: Arc<AgentConfig>,
    client_tcp_stream: TcpStream,
    client_socket_address: SocketAddr,
    rsa_crypto_repo: Arc<R>,
}

impl<R> Tunnel<R>
where
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    pub fn new(
        config: Arc<AgentConfig>,
        client_tcp_stream: TcpStream,
        client_socket_address: SocketAddr,
        rsa_crypto_repo: Arc<R>,
    ) -> Self {
        Self {
            config,
            client_tcp_stream,
            client_socket_address,
            rsa_crypto_repo,
        }
    }

    pub async fn run(self) -> Result<(), CommonError> {
        let client_tcp_stream = self.client_tcp_stream;
        let client_socket_addr = self.client_socket_address;
        let mut protocol = [0u8; 1];
        let peek_size = client_tcp_stream.peek(&mut protocol).await?;
        if peek_size == 0 {
            return Err(CommonError::ConnectionExhausted(client_socket_addr));
        }
        match protocol[0] {
            5 => {
                socks5_protocol_proxy(
                    client_tcp_stream,
                    self.config,
                    self.rsa_crypto_repo,
                    client_socket_addr,
                )
                .await
            }
            4 => {
                socks4_protocol_proxy(
                    client_tcp_stream,
                    self.config,
                    self.rsa_crypto_repo,
                    client_socket_addr,
                )
                .await
            }
            _ => {
                http_protocol_proxy(
                    client_tcp_stream,
                    self.config,
                    self.rsa_crypto_repo,
                    client_socket_addr,
                )
                .await
            }
        }
    }
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
    let tunnel = Tunnel::new(
        config,
        client_tcp_stream,
        client_socket_address,
        rsa_crypto_repo,
    );
    tunnel.run().await
}
