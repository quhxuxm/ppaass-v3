use crate::tunnel::destination::DestinationEdge;
use crate::ProxyConfig;
use futures_util::{SinkExt, StreamExt};
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;

use ppaass_common::{
    AgentTcpConnection, TunnelInitRequest, TunnelInitResponse, UdpRelayDataRequest,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use tokio_util::io::{SinkWriter, StreamReader};

mod destination;

pub struct Tunnel {
    config: Arc<ProxyConfig>,
    agent_tcp_connection: AgentTcpConnection,
    agent_socket_address: SocketAddr,
}

impl Tunnel {
    pub async fn new<T>(
        config: Arc<ProxyConfig>,
        agent_tcp_stream: TcpStream,
        agent_socket_address: SocketAddr,
        rsa_crypto_repo: &T,
    ) -> Result<Self, CommonError>
    where
        T: RsaCryptoRepository + Send + Sync + 'static,
    {
        let agent_tcp_connection =
            AgentTcpConnection::create(agent_tcp_stream, agent_socket_address, rsa_crypto_repo)
                .await?;
        Ok(Self {
            config,
            agent_tcp_connection,
            agent_socket_address,
        })
    }

    async fn initialize_tunnel(
        agent_tcp_connection: &mut AgentTcpConnection,
        agent_socket_address: SocketAddr,
    ) -> Result<DestinationEdge, CommonError> {
        let agent_data = agent_tcp_connection
            .next()
            .await
            .ok_or(CommonError::ConnectionExhausted(agent_socket_address))??;
        let tunnel_init_request: TunnelInitRequest = bincode::deserialize(&agent_data)?;
        match tunnel_init_request {
            TunnelInitRequest::Tcp {
                destination_address,
                keep_alive,
            } => {
                let destination_edge =
                    DestinationEdge::start_tcp(destination_address, keep_alive).await?;
                let tunnel_init_success_response =
                    bincode::serialize(&TunnelInitResponse::Success)?;
                agent_tcp_connection
                    .send(&tunnel_init_success_response)
                    .await?;
                Ok(destination_edge)
            }
            TunnelInitRequest::Udp => Ok(DestinationEdge::start_udp().await?),
        }
    }

    async fn relay(
        agent_tcp_connection: AgentTcpConnection,
        destination_edge: DestinationEdge,
    ) -> Result<(), CommonError> {
        match destination_edge {
            DestinationEdge::Tcp(destination_tcp_endpoint) => {
                let agent_tcp_connection = StreamReader::new(agent_tcp_connection);
                let mut agent_tcp_connection = SinkWriter::new(agent_tcp_connection);
                let destination_tcp_endpoint = StreamReader::new(destination_tcp_endpoint);
                let mut destination_tcp_connection = SinkWriter::new(destination_tcp_endpoint);
                copy_bidirectional(&mut agent_tcp_connection, &mut destination_tcp_connection)
                    .await?;
            }
            DestinationEdge::Udp(destination_udp_endpoint) => {
                let agent_tcp_connection = Arc::new(Mutex::new(agent_tcp_connection));
                loop {
                    let agent_tcp_connection = agent_tcp_connection.clone();
                    let UdpRelayDataRequest {
                        destination_address,
                        source_address,
                        payload,
                    } = match agent_tcp_connection.lock().await.next().await {
                        None => return Ok(()),
                        Some(Err(e)) => return Err(e),
                        Some(Ok(agent_data)) => {
                            bincode::deserialize::<UdpRelayDataRequest>(&agent_data)?
                        }
                    };

                    destination_udp_endpoint
                        .replay(
                            agent_tcp_connection,
                            &payload,
                            source_address,
                            destination_address,
                        )
                        .await?;
                }
            }
        }
        Ok(())
    }
    pub async fn run(mut self) -> Result<(), CommonError> {
        let destination_edge =
            Self::initialize_tunnel(&mut self.agent_tcp_connection, self.agent_socket_address)
                .await?;
        Self::relay(self.agent_tcp_connection, destination_edge).await
    }
}

pub async fn handle_agent_connection<T>(
    config: Arc<ProxyConfig>,
    rsa_crypto_repo: Arc<T>,
    agent_tcp_stream: TcpStream,
    agent_socket_address: SocketAddr,
) -> Result<(), CommonError>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    let tunnel = Tunnel::new(
        config,
        agent_tcp_stream,
        agent_socket_address,
        rsa_crypto_repo.as_ref(),
    )
    .await?;
    tunnel.run().await
}
