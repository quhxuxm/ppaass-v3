mod tcp;
mod udp;
use crate::error::ProxyError;
use crate::tunnel::agent::AgentTcpConnectionWrite;
use futures_util::{SinkExt, StreamExt};
use ppaass_common::crypto::RsaCryptoRepository;
use ppaass_common::error::CommonError;
use ppaass_protocol::{TunnelInitResponse, UnifiedAddress};
pub use tcp::*;
use tokio::task::JoinHandle;
use tokio_util::bytes::BytesMut;
use tracing::error;
pub use udp::*;
pub enum DestinationEdge<R>
where
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    Tcp(DestinationTcpEndpointWrite, JoinHandle<()>),
    Udp(DestinationUdpEndpoint<R>),
}

impl<R> DestinationEdge<R>
where
    R: RsaCryptoRepository + Send + Sync + 'static,
{
    pub async fn start_tcp(
        destination_address: UnifiedAddress,
        keep_alive: bool,
        mut agent_tcp_connection_write: AgentTcpConnectionWrite<R>,
    ) -> Result<Self, CommonError> {
        let destination_tcp_connection =
            DestinationTcpEndpoint::connect(destination_address).await?;
        let tunnel_init_success_response = bincode::serialize(&TunnelInitResponse::Success)?;
        agent_tcp_connection_write
            .send(BytesMut::from_iter(tunnel_init_success_response))
            .await?;
        let (destination_tcp_connection_write, destination_tcp_connection_read) =
            destination_tcp_connection.split();
        let destination_tcp_read_guard = tokio::spawn(async move {
            if let Err(e) = destination_tcp_connection_read
                .forward(agent_tcp_connection_write)
                .await
            {
                error!("Fail to forward destination tcp data to agent tcp connection: {e:?}")
            }
        });

        Ok(Self::Tcp(
            destination_tcp_connection_write,
            destination_tcp_read_guard,
        ))
    }

    pub async fn start_udp(
        agent_tcp_connection_write: AgentTcpConnectionWrite<R>,
    ) -> Result<Self, CommonError> {
        Ok(Self::Udp(DestinationUdpEndpoint::new(
            agent_tcp_connection_write,
        )))
    }
}
