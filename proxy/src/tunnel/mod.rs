use crate::tunnel::agent::{AgentTcpConnection, AgentTcpConnectionRead};
use crate::tunnel::destination::DestinationEdge;
use crate::ProxyConfig;
use futures_util::StreamExt;
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use ppaass_protocol::{TunnelInitRequest, UdpRelayDataRequest};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::error;
mod agent;
mod destination;

pub struct Tunnel<T>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    config: Arc<ProxyConfig>,
    agent_tcp_connection: AgentTcpConnection<T>,
    agent_socket_address: SocketAddr,
}

impl<T> Tunnel<T>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    pub fn new(
        config: Arc<ProxyConfig>,
        agent_tcp_stream: TcpStream,
        agent_socket_address: SocketAddr,
        rsa_crypto_repo: Arc<T>,
    ) -> Self {
        let agent_tcp_connection =
            AgentTcpConnection::new(agent_tcp_stream, agent_socket_address, rsa_crypto_repo);
        Self {
            config,
            agent_tcp_connection,
            agent_socket_address,
        }
    }

    async fn initialize_tunnel(
        agent_tcp_connection: AgentTcpConnection<T>,
        agent_socket_address: SocketAddr,
    ) -> Result<(AgentTcpConnectionRead<T>, DestinationEdge<T>), CommonError> {
        let (agent_tcp_connection_write, mut agent_tcp_connection_read) =
            agent_tcp_connection.split();
        let agent_data = agent_tcp_connection_read
            .next()
            .await
            .ok_or(CommonError::ConnectionExhausted(agent_socket_address))??;
        let tunnel_init_request: TunnelInitRequest = bincode::deserialize(&agent_data)?;
        match tunnel_init_request {
            TunnelInitRequest::Tcp {
                destination_address,
                keep_alive,
            } => Ok((
                agent_tcp_connection_read,
                DestinationEdge::start_tcp(
                    destination_address,
                    keep_alive,
                    agent_tcp_connection_write,
                )
                .await?,
            )),
            TunnelInitRequest::Udp => Ok((
                agent_tcp_connection_read,
                DestinationEdge::start_udp(agent_tcp_connection_write).await?,
            )),
        }
    }

    async fn relay(
        mut agent_tcp_connection_read: AgentTcpConnectionRead<T>,
        destination_edge: DestinationEdge<T>,
    ) -> Result<(), CommonError> {
        match destination_edge {
            DestinationEdge::Tcp(destination_tcp_connection_write, destination_read_guard) => {
                if let Err(e) = agent_tcp_connection_read
                    .forward(destination_tcp_connection_write)
                    .await
                {
                    destination_read_guard.abort();
                    error!("Fail to forward agent tcp connection data to destination tcp connection: {e:?}")
                };
            }
            DestinationEdge::Udp(destination_udp_socket) => loop {
                let UdpRelayDataRequest {
                    destination_address,
                    source_address,
                    payload,
                } = match agent_tcp_connection_read.next().await {
                    None => return Ok(()),
                    Some(Err(e)) => return Err(e),
                    Some(Ok(agent_data)) => {
                        bincode::deserialize::<UdpRelayDataRequest>(&agent_data)?
                    }
                };
                destination_udp_socket
                    .replay(&payload, source_address, destination_address)
                    .await?;
            },
        }
        Ok(())
    }
    pub async fn run(mut self) -> Result<(), CommonError> {
        self.agent_tcp_connection.handshake().await?;
        let (agent_tcp_connection_read, destination_edge) =
            Self::initialize_tunnel(self.agent_tcp_connection, self.agent_socket_address).await?;
        Self::relay(agent_tcp_connection_read, destination_edge).await
    }
}

pub async fn handle_agent_connection<R>(
    config: Arc<ProxyConfig>,
    rsa_crypto_repo: Arc<R>,
    agent_tcp_stream: TcpStream,
    agent_socket_address: SocketAddr,
) -> Result<(), CommonError>
where
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    let tunnel = Tunnel::new(
        config,
        agent_tcp_stream,
        agent_socket_address,
        rsa_crypto_repo,
    );
    tunnel.run().await
}
