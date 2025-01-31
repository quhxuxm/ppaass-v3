use futures_util::SinkExt;

use ppaass_common::error::CommonError;

use ppaass_common::{AgentTcpConnection, UdpRelayDataResponse, UnifiedAddress};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio_tfo::TfoStream;
use tracing::error;
#[derive(Clone)]
pub struct DestinationUdpEndpoint {}

impl DestinationUdpEndpoint {
    pub fn new() -> Self {
        DestinationUdpEndpoint {}
    }

    pub async fn replay(
        &self,
        agent_tcp_connection: Arc<Mutex<AgentTcpConnection<TfoStream>>>,
        data: &[u8],
        source_address: UnifiedAddress,
        destination_address: UnifiedAddress,
    ) -> Result<(), CommonError> {
        let destination_socks_addrs: Vec<SocketAddr> =
            destination_address.clone().try_into().map_err(|e| {
                CommonError::Other(format!(
                    "Fail to convert destination address to socket address: {e}"
                ))
            })?;
        let destination_udp_socket =
            UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))).await?;
        destination_udp_socket
            .send_to(data, destination_socks_addrs.as_slice())
            .await?;
        let agent_tcp_connection = agent_tcp_connection.clone();
        tokio::spawn(async move {
            let mut destination_udp_data = [0u8; 65535];
            let size = match destination_udp_socket.recv(&mut destination_udp_data).await {
                Ok(size) => size,
                Err(e) => {
                    error!("Fail to receive data from destination udp socket: {}", e);
                    return;
                }
            };
            let destination_udp_data = &destination_udp_data[0..size];
            let udp_relay_data_response = UdpRelayDataResponse {
                destination_address,
                source_address,
                payload: destination_udp_data.to_vec(),
            };
            let udp_relay_data_response_bytes = match bincode::serialize(&udp_relay_data_response) {
                Ok(udp_relay_data_response_bytes) => udp_relay_data_response_bytes,
                Err(e) => {
                    error!("Fail to serialize udp relay data: {}", e);
                    return;
                }
            };
            let mut agent_tcp_connection_write = agent_tcp_connection.lock().await;
            if let Err(e) = agent_tcp_connection_write
                .send(&udp_relay_data_response_bytes)
                .await
            {
                error!(
                    "Fail to forward destination udp data to agent tcp connection: {}",
                    e
                );
            };
        });
        Ok(())
    }
}
