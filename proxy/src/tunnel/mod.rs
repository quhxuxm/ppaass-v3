use crate::tunnel::destination::DestinationEdge;
use crate::ProxyConfig;
use futures_util::StreamExt;
use ppaass_common::error::CommonError;

use crate::user::ForwardProxyUserRepository;
use ppaass_common::server::{ServerState, ServerTcpStream};
use ppaass_common::user::repo::fs::FileSystemUserInfoRepository;
use ppaass_common::{
    AgentTcpConnection, AgentTcpConnectionTunnelCtlState, ProxyTcpConnectionInfoSelector,
    TunnelInitFailureReason, TunnelInitRequest, TunnelInitResponse, UdpRelayDataRequest,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{copy_bidirectional, copy_bidirectional_with_sizes};
use tokio::sync::Mutex;
use tokio_tfo::TfoStream;
use tokio_util::io::{SinkWriter, StreamReader};
use tracing::debug;
mod destination;

pub struct Tunnel {
    config: Arc<ProxyConfig>,
    agent_tcp_connection: AgentTcpConnection<AgentTcpConnectionTunnelCtlState<TfoStream>>,
    agent_socket_address: SocketAddr,
    server_state: Arc<ServerState>,
}

impl Tunnel {
    pub async fn new(
        config: Arc<ProxyConfig>,
        server_state: Arc<ServerState>,
        agent_tcp_stream: TfoStream,
        agent_socket_address: SocketAddr,
    ) -> Result<Self, CommonError> {
        let user_repo = server_state
            .get_value::<Arc<FileSystemUserInfoRepository>>()
            .ok_or(CommonError::Other(format!(
                "Fail to get user crypto repository for agent: {agent_socket_address}"
            )))?;
        let agent_tcp_connection = AgentTcpConnection::create(
            agent_tcp_stream,
            agent_socket_address,
            user_repo.as_ref(),
            config.agent_frame_buffer_size(),
        )
        .await?;
        Ok(Self {
            config,
            server_state,
            agent_tcp_connection,
            agent_socket_address,
        })
    }

    async fn initialize_tunnel(
        tunnel_init_request: TunnelInitRequest,
        agent_socket_address: SocketAddr,
        config: &ProxyConfig,
        server_state: &ServerState,
    ) -> Result<DestinationEdge, CommonError> {
        match tunnel_init_request {
            TunnelInitRequest::Tcp {
                destination_address,
                keep_alive,
            } => match config.forward() {
                None => {
                    debug!("[START TCP] Begin to initialize tunnel for agent: {agent_socket_address:?}");
                    let destination_edge = DestinationEdge::start_tcp(
                        destination_address,
                        keep_alive,
                        config.destination_connect_timeout(),
                    )
                    .await?;
                    Ok(destination_edge)
                }
                Some(forward_config) => {
                    debug!("[START FORWARD] Begin to initialize tunnel for agent: {agent_socket_address:?}");
                    let forward_rsa_crypto_repo=server_state.get_value::<Arc<ForwardProxyUserRepository>>().ok_or(CommonError::Other("Proxy configured as forward but no forward user crypto repository configured.".to_string()))?;
                    let destination_edge = DestinationEdge::start_forward(
                        server_state,
                        forward_config.select_proxy_tcp_connection_info()?,
                        forward_rsa_crypto_repo.as_ref(),
                        destination_address,
                    )
                    .await?;
                    Ok(destination_edge)
                }
            },
            TunnelInitRequest::Udp => {
                debug!(
                    "[START UDP] Begin to initialize tunnel for agent: {agent_socket_address:?}"
                );
                Ok(DestinationEdge::start_udp().await?)
            }
        }
    }

    pub async fn run(mut self) -> Result<(), CommonError> {
        let tunnel_init_request = self.agent_tcp_connection.wait_tunnel_init().await?;
        match Self::initialize_tunnel(
            tunnel_init_request,
            self.agent_socket_address,
            self.config.as_ref(),
            self.server_state.as_ref(),
        )
        .await
        {
            Err(e) => {
                self.agent_tcp_connection
                    .response_tcp_tunnel_init(TunnelInitResponse::Failure(
                        TunnelInitFailureReason::InitWithDestinationFail,
                    ))
                    .await?;
                Err(e)
            }
            Ok(destination_edge) => match destination_edge {
                DestinationEdge::Tcp(destination_tcp_endpoint) => {
                    let mut agent_tcp_connection = self
                        .agent_tcp_connection
                        .response_tcp_tunnel_init(TunnelInitResponse::Success)
                        .await?;
                    let agent_socket_address = agent_tcp_connection.agent_socket_address();
                    let destination_address =
                        destination_tcp_endpoint.destination_address().clone();
                    let destination_tcp_endpoint = StreamReader::new(destination_tcp_endpoint);
                    let mut destination_tcp_connection = SinkWriter::new(destination_tcp_endpoint);
                    debug!("[PROXYING] Going to copy bidirectional between agent [{agent_socket_address}] and destination [{destination_address}]");
                    let (agent_data_size, destination_data_size) = copy_bidirectional_with_sizes(
                        &mut agent_tcp_connection,
                        &mut destination_tcp_connection,
                        self.config.proxy_to_destination_data_relay_buffer_size(),
                        self.config.destination_to_proxy_data_relay_buffer_size(),
                    )
                    .await?;
                    debug!("[PROXYING] Copy data between agent and destination, agent data size: {agent_data_size}, destination data size: {destination_data_size}");
                    Ok(())
                }
                DestinationEdge::Forward(mut forward_proxy_tcp_connection) => {
                    let mut agent_tcp_connection = self
                        .agent_tcp_connection
                        .response_tcp_tunnel_init(TunnelInitResponse::Success)
                        .await?;
                    let agent_socket_address = agent_tcp_connection.agent_socket_address();
                    let proxy_socket_address = forward_proxy_tcp_connection.proxy_socket_address();
                    debug!("[FORWARDING] Going to copy bidirectional between agent [{agent_socket_address}] and proxy [{proxy_socket_address}]");
                    let (agent_data_size, proxy_data_size) = copy_bidirectional(
                        &mut agent_tcp_connection,
                        &mut forward_proxy_tcp_connection,
                    )
                    .await?;
                    debug!("[FORWARDING] Copy data between agent and proxy, agent data size: {agent_data_size}, proxy data size: {proxy_data_size}");
                    Ok(())
                }
                DestinationEdge::Udp(destination_udp_endpoint) => {
                    let agent_tcp_connection = self
                        .agent_tcp_connection
                        .response_udp_tunnel_init(TunnelInitResponse::Success)
                        .await?;
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
            },
        }
    }
}

pub async fn handle_agent_connection(
    config: Arc<ProxyConfig>,
    server_state: Arc<ServerState>,
    agent_tcp_stream: ServerTcpStream,
    agent_socket_address: SocketAddr,
) -> Result<(), CommonError> {
    let ServerTcpStream::TfoStream(agent_tcp_stream) = agent_tcp_stream else {
        return Err(CommonError::Other(format!(
            "Proxy server should use tfo stream: {agent_socket_address}"
        )));
    };
    let tunnel = Tunnel::new(config, server_state, agent_tcp_stream, agent_socket_address).await?;
    tunnel.run().await
}
