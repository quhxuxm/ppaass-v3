mod pool;
use crate::connection::codec::{
    HandshakeRequestEncoder, HandshakeResponseDecoder, TunnelControlResponseRequestCodec,
};
use crate::connection::CryptoLengthDelimitedFramed;
use crate::error::CommonError;
use crate::user::repo::fs::USER_INFO_ADDITION_INFO_PROXY_SERVERS;
use crate::user::UserInfo;
use crate::{
    parse_to_socket_addresses, random_generate_encryption, rsa_decrypt_encryption,
    rsa_encrypt_encryption, FramedConnection,
};
use bytes::BytesMut;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
pub use pool::*;
use ppaass_protocol::{
    Encryption, HandshakeRequest, HandshakeResponse, HeartbeatRequest, TunnelControlRequest,
    TunnelControlResponse, TunnelInitFailureReason, TunnelInitRequest, TunnelInitResponse,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::{Framed, FramedParts};
use tokio_util::io::{SinkWriter, StreamReader};
use tracing::debug;
#[derive(Debug, Clone)]
pub struct ProxyTcpConnectionInfo {
    proxy_address: SocketAddr,
    authentication: String,
}
impl ProxyTcpConnectionInfo {
    pub fn new(proxy_address: SocketAddr, authentication: String) -> Self {
        Self {
            proxy_address,
            authentication,
        }
    }
    pub fn authentication(&self) -> &str {
        &self.authentication
    }
    pub fn proxy_address(&self) -> SocketAddr {
        self.proxy_address
    }
}

pub struct ProxyTcpConnectionNewState {}
pub struct ProxyTcpConnectionTunnelCtlState {
    tunnel_ctl_response_request_framed: Framed<TcpStream, TunnelControlResponseRequestCodec>,
    proxy_encryption: Arc<Encryption>,
    agent_encryption: Arc<Encryption>,
}

fn select_proxy_tcp_connection_info(
    username: &str,
    user_info: &UserInfo,
) -> Result<ProxyTcpConnectionInfo, CommonError> {
    let proxy_addresses = user_info
        .get_additional_info::<Vec<String>>(USER_INFO_ADDITION_INFO_PROXY_SERVERS)
        .ok_or(CommonError::Other(format!(
            "No proxy servers defined in user info configuration: {user_info:?}"
        )))?;
    let proxy_addresses = parse_to_socket_addresses(proxy_addresses.iter())?;

    let select_index = rand::random::<u64>() % proxy_addresses.len() as u64;
    let proxy_address = proxy_addresses[select_index as usize];

    Ok(ProxyTcpConnectionInfo::new(
        proxy_address,
        username.to_owned(),
    ))
}

impl FramedConnection<ProxyTcpConnectionNewState> {
    pub async fn create(
        username: &str,
        user_info: &UserInfo,
        frame_buffer_size: usize,
        connect_timeout: u64,
    ) -> Result<FramedConnection<ProxyTcpConnectionTunnelCtlState>, CommonError> {
        let proxy_tcp_connection_info = select_proxy_tcp_connection_info(username, user_info)?;
        let proxy_tcp_stream = timeout(
            Duration::from_secs(connect_timeout),
            TcpStream::connect(proxy_tcp_connection_info.proxy_address()),
        )
        .await??;
        proxy_tcp_stream.set_nodelay(true)?;
        proxy_tcp_stream.set_linger(None)?;
        let proxy_socket_address = proxy_tcp_stream.peer_addr()?;
        let agent_encryption = random_generate_encryption();
        let encrypt_agent_encryption =
            rsa_encrypt_encryption(&agent_encryption, user_info.rsa_crypto())?;
        let mut handshake_request_framed =
            Framed::new(proxy_tcp_stream, HandshakeRequestEncoder::new());
        let handshake_request = HandshakeRequest {
            authentication: proxy_tcp_connection_info.authentication().to_owned(),
            encryption: encrypt_agent_encryption.into_owned(),
        };
        debug!("Begin to send handshake request to proxy: {handshake_request:?}");
        handshake_request_framed.send(handshake_request).await?;
        debug!("Success to send handshake request to proxy: {proxy_socket_address:?}");
        debug!("Begin to receive handshake response from proxy: {proxy_socket_address:?}");
        let FramedParts {
            io: proxy_tcp_stream,
            ..
        } = handshake_request_framed.into_parts();
        let mut handshake_response_framed =
            Framed::new(proxy_tcp_stream, HandshakeResponseDecoder::new());
        let HandshakeResponse {
            encryption: proxy_encryption,
        } = handshake_response_framed
            .next()
            .await
            .ok_or(CommonError::ConnectionExhausted(proxy_socket_address))??;
        debug!("Success to receive handshake response from proxy: {proxy_socket_address:?}");
        let proxy_encryption =
            rsa_decrypt_encryption(&proxy_encryption, user_info.rsa_crypto())?.into_owned();
        let FramedParts {
            io: proxy_tcp_stream,
            ..
        } = handshake_response_framed.into_parts();
        let socket_address = proxy_tcp_stream.peer_addr()?;
        let proxy_encryption = Arc::new(proxy_encryption);
        let agent_encryption = Arc::new(agent_encryption);
        Ok(FramedConnection {
            state: ProxyTcpConnectionTunnelCtlState {
                proxy_encryption: proxy_encryption.clone(),
                agent_encryption: agent_encryption.clone(),
                tunnel_ctl_response_request_framed: Framed::with_capacity(
                    proxy_tcp_stream,
                    TunnelControlResponseRequestCodec::new(proxy_encryption, agent_encryption),
                    frame_buffer_size,
                ),
            },
            socket_address,
            frame_buffer_size,
        })
    }
}

impl FramedConnection<ProxyTcpConnectionTunnelCtlState> {
    pub async fn tunnel_init(
        mut self,
        tunnel_init_request: TunnelInitRequest,
    ) -> Result<
        FramedConnection<
            SinkWriter<StreamReader<CryptoLengthDelimitedFramed<TcpStream>, BytesMut>>,
        >,
        CommonError,
    > {
        let tunnel_ctl_request = TunnelControlRequest::TunnelInit(tunnel_init_request);
        self.state
            .tunnel_ctl_response_request_framed
            .send(tunnel_ctl_request)
            .await?;
        let mut times_to_receive_heartbeat = 0;
        loop {
            let tunnel_ctl_response = self
                .state
                .tunnel_ctl_response_request_framed
                .next()
                .await
                .ok_or(CommonError::ConnectionExhausted(self.socket_address))??;
            match tunnel_ctl_response {
                TunnelControlResponse::Heartbeat(heartbeat) => {
                    debug!("Receive heartbeat response from proxy connection: {heartbeat:?}");
                    times_to_receive_heartbeat += 1;
                    if times_to_receive_heartbeat >= 3 {
                        return Err(CommonError::Other(
                            "Receive too many heartbeats when initialize tunnel.".to_string(),
                        ));
                    }
                    continue;
                }
                TunnelControlResponse::TunnelInit(tunnel_init_response) => {
                    return match tunnel_init_response {
                        TunnelInitResponse::Success => {
                            let FramedParts { io, .. } =
                                self.state.tunnel_ctl_response_request_framed.into_parts();
                            Ok(FramedConnection {
                                socket_address: self.socket_address,
                                frame_buffer_size: self.frame_buffer_size,
                                state: SinkWriter::new(StreamReader::new(
                                    CryptoLengthDelimitedFramed::new(
                                        io,
                                        self.state.proxy_encryption,
                                        self.state.agent_encryption,
                                        self.frame_buffer_size,
                                    ),
                                )),
                            })
                        }
                        TunnelInitResponse::Failure(TunnelInitFailureReason::AuthenticateFail) => {
                            Err(CommonError::Other(format!(
                                "Tunnel init fail on authenticate: {tunnel_init_response:?}",
                            )))
                        }
                        TunnelInitResponse::Failure(
                            TunnelInitFailureReason::InitWithDestinationFail,
                        ) => Err(CommonError::Other(format!(
                            "Tunnel init fail on connect destination: {tunnel_init_response:?}",
                        ))),
                    };
                }
            }
        }
    }
    pub async fn heartbeat(&mut self, timeout_seconds: u64) -> Result<i64, CommonError> {
        let start_time = Utc::now();

        let heartbeat_request = TunnelControlRequest::Heartbeat(HeartbeatRequest::new());
        self.state
            .tunnel_ctl_response_request_framed
            .send(heartbeat_request)
            .await?;
        let tunnel_ctl_response = timeout(
            Duration::from_secs(timeout_seconds),
            self.state.tunnel_ctl_response_request_framed.next(),
        )
        .await?
        .ok_or(CommonError::ConnectionExhausted(self.socket_address))??;
        match tunnel_ctl_response {
            TunnelControlResponse::Heartbeat(heartbeat_response) => {
                let end_time = Utc::now();
                let check_duration = end_time
                    .signed_duration_since(start_time)
                    .num_milliseconds();
                debug!("Receive heartbeat response from proxy connection: {heartbeat_response:?}");
                Ok(check_duration)
            }
            TunnelControlResponse::TunnelInit(_) => Err(CommonError::Other(format!(
                "Receive tunnel init response from proxy connection: {}",
                self.socket_address
            ))),
        }
    }

    pub async fn close(&mut self) -> Result<(), CommonError> {
        self.state.tunnel_ctl_response_request_framed.close().await
    }
}
