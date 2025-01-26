mod client;

use crate::config::AgentConfig;
pub use client::*;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::debug;
pub struct Tunnel<T>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    config: Arc<AgentConfig>,
    client_tcp_stream: TcpStream,
    client_socket_address: SocketAddr,
    rsa_crypto_repo: Arc<T>,
}

impl<T> Tunnel<T>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    pub fn new(
        config: Arc<AgentConfig>,
        client_tcp_stream: TcpStream,
        client_socket_address: SocketAddr,
        rsa_crypto_repo: Arc<T>,
    ) -> Self {
        Self {
            config,
            client_tcp_stream,
            client_socket_address,
            rsa_crypto_repo,
        }
    }

    pub async fn run(mut self) -> Result<(), CommonError> {
        let client_tcp_stream = self.client_tcp_stream;
        let client_socket_addr = self.client_socket_address;
        let mut protocol = [0u8; 1];
        let peek_size = client_tcp_stream.peek(&mut protocol).await?;
        if peek_size == 0 {
            return Err(CommonError::ConnectionExhausted(client_socket_addr));
        }
        match protocol[0] {
            5 => {
                debug!("Client connect to agent with socks 5 protocol: {client_socket_addr}");
                unimplemented!("Socks 5 protocol is not yet implemented");
            }
            4 => {
                debug!("Client connect to agent with socks 4 protocol: {client_socket_addr}");
                unimplemented!("Socks 4 protocol is not yet implemented");
            }
            _ => {
                let client_http_connection = ClientHttpConnection::new(client_tcp_stream);
                client_http_connection.exec().await
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
