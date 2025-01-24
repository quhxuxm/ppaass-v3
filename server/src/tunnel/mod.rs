use crate::error::ServerError;
use crate::tunnel::agent::AgentConnection;
use crate::tunnel::destination::DestinationTcpConnection;
use crate::{ServerConfig, ServerRsaCryptoRepo};
use futures_util::StreamExt;
use ppaass_protocol::{AgentRequestPacket, TunnelInitRequest};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
mod agent;
mod destination;
pub struct Tunnel<'r> {
    config: Arc<ServerConfig>,
    agent_tcp_connection: AgentConnection<'r, ServerRsaCryptoRepo>,
    agent_socket_address: SocketAddr,
    destination_tcp_connection: Option<DestinationTcpConnection>,
}

impl<'r> Tunnel<'r> {
    pub fn new(
        config: Arc<ServerConfig>,
        agent_tcp_stream: TcpStream,
        agent_socket_address: SocketAddr,
        rsa_crypto_repo: &'r ServerRsaCryptoRepo,
    ) -> Self {
        let agent_tcp_connection =
            AgentConnection::new(agent_tcp_stream, agent_socket_address, rsa_crypto_repo);
        Self {
            config,
            agent_tcp_connection,
            agent_socket_address,
            destination_tcp_connection: None,
        }
    }

    pub async fn run(mut self) -> Result<(), ServerError> {
        self.agent_tcp_connection.handshake().await?;
        let (agent_connection_write, mut agent_connection_read) = self.agent_tcp_connection.split();
        loop {
            let agent_request_package_bytes = agent_connection_read.next().await.ok_or(
                ServerError::AgentConnectionExhausted(self.agent_socket_address),
            )??;
            let agent_request_packet: AgentRequestPacket =
                bincode::deserialize(&agent_request_package_bytes)?;
            match agent_request_packet {
                AgentRequestPacket::Init(tunnel_init_request) => match tunnel_init_request {
                    TunnelInitRequest::Tcp {
                        destination_address,
                        keep_alive,
                    } => {
                        let destination_tcp_connection =
                            DestinationTcpConnection::connect(destination_address).await?;
                        let (destination_connection_write, destination_connection_read) =
                            destination_tcp_connection.split();
                    }
                    TunnelInitRequest::Udp => {}
                },
                AgentRequestPacket::Relay(_) => {}
            }
        }
        Ok(())
    }
}
