use crate::config::AgentConfig;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use ppaass_common::ProxyTcpConnectionPool;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::debug;
pub async fn socks4_protocol_proxy<R>(
    _client_tcp_stream: TcpStream,
    _config: Arc<AgentConfig>,
    _rsa_crypto_repo: Arc<R>,
    client_socket_addr: SocketAddr,
    _proxy_tcp_connection_pool: Option<Arc<ProxyTcpConnectionPool<AgentConfig, AgentConfig, R>>>,
) -> Result<(), CommonError>
where
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    debug!("Client connect to agent with socks 4 protocol: {client_socket_addr}");
    unimplemented!("Socks 4 protocol is not yet implemented");
}
