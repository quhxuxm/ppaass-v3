use crate::tunnel::destination::DestinationEdge;
use crate::ProxyConfig;
use futures_util::{SinkExt, StreamExt};
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;

use ppaass_common::{
    AgentTcpConnection, ProxyTcpConnectionInfoSelector, ProxyTcpConnectionPool, TunnelInitRequest,
    TunnelInitResponse, UdpRelayDataRequest,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{copy_bidirectional, copy_bidirectional_with_sizes};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use tokio_util::io::{SinkWriter, StreamReader};
use tracing::debug;
mod destination;

pub struct Tunnel<'a, T>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    config: Arc<ProxyConfig>,
    agent_tcp_connection: AgentTcpConnection,
    agent_socket_address: SocketAddr,
    forward_rsa_crypto_repo: Option<&'a T>,
    forward_proxy_tcp_connection_pool:
        Option<Arc<ProxyTcpConnectionPool<ProxyConfig, ProxyConfig, T>>>,
}

impl<'a, T> Tunnel<'a, T>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    pub async fn new(
        config: Arc<ProxyConfig>,
        agent_tcp_stream: TcpStream,
        agent_socket_address: SocketAddr,
        rsa_crypto_repo: &'a T,
        forward_rsa_crypto_repo: Option<&'a T>,
        forward_proxy_tcp_connection_pool: Option<
            Arc<ProxyTcpConnectionPool<ProxyConfig, ProxyConfig, T>>,
        >,
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
            forward_rsa_crypto_repo,
            forward_proxy_tcp_connection_pool,
        })
    }

    async fn initialize_tunnel(
        agent_tcp_connection: &mut AgentTcpConnection,
        agent_socket_address: SocketAddr,
        config: &ProxyConfig,
        forward_rsa_crypto_repo: Option<&T>,
        forward_proxy_tcp_connection_pool: Option<
            Arc<ProxyTcpConnectionPool<ProxyConfig, ProxyConfig, T>>,
        >,
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
            } => match config.forward_proxies() {
                None => {
                    let destination_edge =
                        DestinationEdge::start_tcp(destination_address, keep_alive).await?;
                    let tunnel_init_success_response =
                        bincode::serialize(&TunnelInitResponse::Success)?;
                    agent_tcp_connection
                        .send(&tunnel_init_success_response)
                        .await?;
                    Ok(destination_edge)
                }
                Some(_) => {
                    let destination_edge = DestinationEdge::start_forward(
                        config.select_proxy_tcp_connection_info()?,
                        forward_rsa_crypto_repo.ok_or(CommonError::Other(
                            "Forward rsa crypto repo not exist".to_string(),
                        ))?,
                        destination_address,
                        forward_proxy_tcp_connection_pool,
                    )
                    .await?;
                    let tunnel_init_success_response =
                        bincode::serialize(&TunnelInitResponse::Success)?;
                    agent_tcp_connection
                        .send(&tunnel_init_success_response)
                        .await?;
                    Ok(destination_edge)
                }
            },
            TunnelInitRequest::Udp => Ok(DestinationEdge::start_udp().await?),
        }
    }

    async fn relay(
        agent_tcp_connection: AgentTcpConnection,
        destination_edge: DestinationEdge,
        config: &ProxyConfig,
    ) -> Result<(), CommonError> {
        match destination_edge {
            DestinationEdge::Forward(proxy_tcp_connection) => {
                let agent_socket_address = agent_tcp_connection.agent_socket_address();
                let proxy_socket_address = proxy_tcp_connection.proxy_socket_address();
                let agent_tcp_connection = StreamReader::new(agent_tcp_connection);
                let mut agent_tcp_connection = SinkWriter::new(agent_tcp_connection);
                let proxy_tcp_connection = StreamReader::new(proxy_tcp_connection);
                let mut proxy_tcp_connection = SinkWriter::new(proxy_tcp_connection);
                debug!("[FORWARDING] Going to copy bidirectional between agent [{agent_socket_address}] and proxy [{proxy_socket_address}]");
                let (agent_data_size, proxy_data_size) =
                    copy_bidirectional(&mut agent_tcp_connection, &mut proxy_tcp_connection)
                        .await?;
                debug!("[FORWARDING] Copy data between agent and proxy, agent data size: {agent_data_size}, proxy data size: {proxy_data_size}");
            }
            DestinationEdge::Tcp(destination_tcp_endpoint) => {
                let agent_socket_address = agent_tcp_connection.agent_socket_address();
                let destination_address = destination_tcp_endpoint.destination_address().clone();
                let agent_tcp_connection = StreamReader::new(agent_tcp_connection);
                let mut agent_tcp_connection = SinkWriter::new(agent_tcp_connection);
                let destination_tcp_endpoint = StreamReader::new(destination_tcp_endpoint);
                let mut destination_tcp_connection = SinkWriter::new(destination_tcp_endpoint);
                debug!("[PROXYING] Going to copy bidirectional between agent [{agent_socket_address}] and destination [{destination_address}]");
                let (agent_data_size, destination_data_size) = copy_bidirectional_with_sizes(
                    &mut agent_tcp_connection,
                    &mut destination_tcp_connection,
                    config.proxy_to_destination_data_relay_buffer_size(),
                    config.destination_to_proxy_data_relay_buffer_size(),
                )
                .await?;
                debug!("[PROXYING] Copy data between agent and destination, agent data size: {agent_data_size}, destination data size: {destination_data_size}");
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
        let destination_edge = Self::initialize_tunnel(
            &mut self.agent_tcp_connection,
            self.agent_socket_address,
            self.config.as_ref(),
            self.forward_rsa_crypto_repo,
            self.forward_proxy_tcp_connection_pool,
        )
        .await?;
        Self::relay(
            self.agent_tcp_connection,
            destination_edge,
            self.config.as_ref(),
        )
        .await
    }
}

pub async fn handle_agent_connection<T>(
    config: Arc<ProxyConfig>,
    rsa_crypto_repo: Arc<T>,
    forward_rsa_crypto_repo: Option<Arc<T>>,
    agent_tcp_stream: TcpStream,
    agent_socket_address: SocketAddr,
    forward_proxy_tcp_connection_pool: Option<
        Arc<ProxyTcpConnectionPool<ProxyConfig, ProxyConfig, T>>,
    >,
) -> Result<(), CommonError>
where
    T: RsaCryptoRepository + Send + Sync + 'static,
{
    let tunnel = Tunnel::new(
        config,
        agent_tcp_stream,
        agent_socket_address,
        rsa_crypto_repo.as_ref(),
        forward_rsa_crypto_repo.as_deref(),
        forward_proxy_tcp_connection_pool,
    )
    .await?;
    tunnel.run().await
}
