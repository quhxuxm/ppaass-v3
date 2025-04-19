mod tcp;
use crate::config::ForwardConfig;
use ppaass_common::config::RetrieveConnectionConfig;
use ppaass_common::error::CommonError;
use ppaass_common::server::ServerState;
use ppaass_common::user::UserInfo;
use ppaass_common::{
    CryptoLengthDelimitedFramed, FramedConnection, ProxyTcpConnectionNewState,
    ProxyTcpConnectionPool, TunnelInitRequest, UnifiedAddress,
};
use std::sync::Arc;
pub use tcp::*;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_util::bytes::BytesMut;
use tokio_util::io::{SinkWriter, StreamReader};
pub enum DestinationEdge {
    Direct(DestinationTcpEndpoint),
    Forward(
        FramedConnection<
            SinkWriter<StreamReader<CryptoLengthDelimitedFramed<TcpStream>, BytesMut>>,
        >,
    ),
}

impl DestinationEdge {
    pub async fn start_direct(
        destination_address: UnifiedAddress,
        keep_alive: bool,
        connect_timeout: u64,
    ) -> Result<Self, CommonError> {
        let destination_tcp_connection =
            DestinationTcpEndpoint::connect(destination_address, keep_alive, connect_timeout)
                .await?;
        Ok(Self::Direct(destination_tcp_connection))
    }

    pub async fn start_forward<T: RetrieveConnectionConfig>(
        server_state: &ServerState,
        forward_config: &T,
        destination_address: UnifiedAddress,
    ) -> Result<Self, CommonError> {
        let (username, user_info) = server_state
            .get_value::<(String, Arc<RwLock<UserInfo>>)>()
            .ok_or(CommonError::Other("Can not find forward user".to_owned()))?;
        let proxy_tcp_connection_pool =
            match server_state.get_value::<Arc<ProxyTcpConnectionPool<ForwardConfig>>>() {
                None => {
                    let user_info = user_info.read().await;
                    FramedConnection::<ProxyTcpConnectionNewState>::create(
                        &username,
                        &user_info,
                        forward_config.frame_size(),
                        forward_config.connect_timeout(),
                    )
                    .await?
                }
                Some(pool) => pool.take_proxy_connection().await?,
            };

        let proxy_tcp_connection = proxy_tcp_connection_pool
            .tunnel_init(TunnelInitRequest {
                destination_address: destination_address.clone(),
                keep_alive: false,
            })
            .await?;

        Ok(Self::Forward(proxy_tcp_connection))
    }
}
