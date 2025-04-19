mod client;
pub use client::*;
use ppaass_common::config::RetrieveConnectionConfig;
use ppaass_common::error::CommonError;
use ppaass_common::server::ServerState;
use ppaass_common::user::UserInfo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{net::TcpStream, sync::RwLock};
use tracing::{debug, error};
const SOCKS5_VERSION: u8 = 0x05;
const SOCKS4_VERSION: u8 = 0x04;

pub async fn handle_client_connection<T: RetrieveConnectionConfig>(
    config: Arc<T>,
    server_state: Arc<ServerState>,
    client_tcp_stream: TcpStream,
    client_socket_address: SocketAddr,
) -> Result<(), CommonError> {
    let client_tcp_stream = client_tcp_stream;
    let client_socket_addr = client_socket_address;
    let mut protocol = [0u8; 1];
    let peek_size = client_tcp_stream.peek(&mut protocol).await?;
    if peek_size == 0 {
        error!("Client tcp stream exhausted: {client_socket_addr}");
        return Err(CommonError::ConnectionExhausted(client_socket_addr));
    }
    let (username, user_info) = server_state
        .get_value::<(String, Arc<RwLock<UserInfo>>)>()
        .ok_or(CommonError::Other("Can not get user info".to_owned()))?
        .clone();
    match protocol[0] {
        SOCKS5_VERSION => {
            debug!("Client tcp stream using socks5 ppaass-v3-protocol: {client_socket_addr}");
            socks5_protocol_proxy(
                client_tcp_stream,
                client_socket_addr,
                config.as_ref(),
                &username,
                user_info,
                server_state,
            )
            .await
        }
        SOCKS4_VERSION => {
            debug!("Client tcp stream using socks4 ppaass-v3-protocol: {client_socket_addr}");
            Err(CommonError::Other(
                "Socks4 proxy is not supported".to_owned(),
            ))
        }
        _ => {
            debug!("Client tcp stream using http ppaass-v3-protocol: {client_socket_addr}");
            http_protocol_proxy(
                client_tcp_stream,
                client_socket_addr,
                config.as_ref(),
                &username,
                user_info,
                server_state,
            )
            .await
        }
    }
}
