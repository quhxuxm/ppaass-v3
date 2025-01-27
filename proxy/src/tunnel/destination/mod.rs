mod tcp;
mod udp;
use ppaass_common::error::CommonError;
use ppaass_common::{AgentTcpConnection, UnifiedAddress};
pub use tcp::*;
pub use udp::*;
pub enum DestinationEdge {
    Tcp(DestinationTcpEndpoint),
    Udp(DestinationUdpEndpoint),
}

impl DestinationEdge {
    pub async fn start_tcp(
        destination_address: UnifiedAddress,
        keep_alive: bool,
    ) -> Result<Self, CommonError> {
        let destination_tcp_connection =
            DestinationTcpEndpoint::connect(destination_address, keep_alive).await?;
        Ok(Self::Tcp(destination_tcp_connection))
    }

    pub async fn start_udp() -> Result<Self, CommonError> {
        Ok(Self::Udp(DestinationUdpEndpoint::new()))
    }
}
