use crate::config::AgentConfig;
use ppaass_common::error::CommonError;
use ppaass_common::server::ServerState;
use ppaass_common::user::UserInfo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tfo::TfoStream;
use tracing::debug;
pub async fn socks4_protocol_proxy(
    _client_tcp_stream: TfoStream,
    client_socket_addr: SocketAddr,
    _config: &AgentConfig,
    _user_info: Arc<RwLock<UserInfo>>,
    _server_state: Arc<ServerState>,
) -> Result<(), CommonError> {
    debug!("Client connect to agent with socks 4 protocol: {client_socket_addr}");
    Err(CommonError::Other(
        "Socks4 proxy is not supported".to_owned(),
    ))
}
