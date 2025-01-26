use crate::config::AgentConfig;
use ppaass_common::error::CommonError;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::debug;
pub async fn socks5_protocol_proxy<R>(
    client_tcp_stream: TcpStream,
    config: Arc<AgentConfig>,
    rsa_crypto_repo: Arc<R>,
    client_socket_addr: SocketAddr,
) -> Result<(), CommonError> {
    debug!("Client connect to agent with socks 5 protocol: {client_socket_addr}");
    unimplemented!("Socks 5 protocol is not yet implemented");
}
