use crate::connection::codec::{
    HandshakeRequestEncoder, HandshakeResponseDecoder, TunnelControlResponseRequestCodec,
};

use crate::connection::CryptoLengthDelimitedFramed;
use crate::error::CommonError;

use crate::user::{UserInfo, UserInfoRepository};
use crate::{random_generate_encryption, rsa_decrypt_encryption, rsa_encrypt_encryption};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use ppaass_protocol::{
    Encryption, HandshakeRequest, HandshakeResponse, HeartbeatRequest, TunnelControlRequest,
    TunnelControlResponse, TunnelInitFailureReason, TunnelInitRequest, TunnelInitResponse,
};

use std::fmt::{Debug, Formatter};
use std::io::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::pin;
use tokio::time::timeout;
use tokio_tfo::TfoStream;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Framed, FramedParts};
use tokio_util::io::{SinkWriter, StreamReader};
use tracing::debug;
pub struct ProxyTcpConnectionNewState {}
pub struct ProxyTcpConnectionTunnelCtlState {
    tunnel_ctl_response_request_framed: Framed<TfoStream, TunnelControlResponseRequestCodec>,
    proxy_encryption: Arc<Encryption>,
    agent_encryption: Arc<Encryption>,
}

pub struct ProxyTcpConnectionRelayState {
    crypto_tcp_read_write:
        SinkWriter<StreamReader<CryptoLengthDelimitedFramed<TfoStream>, BytesMut>>,
}
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
pub struct ProxyTcpConnection<T> {
    proxy_socket_address: SocketAddr,
    frame_buffer_size: usize,
    state: T,
}
impl<T> Debug for ProxyTcpConnection<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProxyTcpConnection: {}", self.proxy_socket_address)
    }
}
impl<T> ProxyTcpConnection<T> {
    pub fn proxy_socket_address(&self) -> SocketAddr {
        self.proxy_socket_address
    }
}
impl ProxyTcpConnection<ProxyTcpConnectionNewState> {
    pub async fn create(
        proxy_tcp_connection_info: ProxyTcpConnectionInfo,
        user_info: &UserInfo,
        frame_buffer_size: usize,
        connect_timeout: u64,
    ) -> Result<ProxyTcpConnection<ProxyTcpConnectionTunnelCtlState>, CommonError> {
        let proxy_tcp_stream = timeout(
            Duration::from_secs(connect_timeout),
            TfoStream::connect(proxy_tcp_connection_info.proxy_address()),
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
        let proxy_socket_address = proxy_tcp_stream.peer_addr()?;
        let proxy_encryption = Arc::new(proxy_encryption);
        let agent_encryption = Arc::new(agent_encryption);
        Ok(ProxyTcpConnection {
            state: ProxyTcpConnectionTunnelCtlState {
                proxy_encryption: proxy_encryption.clone(),
                agent_encryption: agent_encryption.clone(),
                tunnel_ctl_response_request_framed: Framed::with_capacity(
                    proxy_tcp_stream,
                    TunnelControlResponseRequestCodec::new(proxy_encryption, agent_encryption),
                    frame_buffer_size,
                ),
            },
            proxy_socket_address,
            frame_buffer_size,
        })
    }
}
impl ProxyTcpConnection<ProxyTcpConnectionTunnelCtlState> {
    pub async fn tunnel_init(
        mut self,
        tunnel_init_request: TunnelInitRequest,
    ) -> Result<ProxyTcpConnection<ProxyTcpConnectionRelayState>, CommonError> {
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
                .ok_or(CommonError::ConnectionExhausted(self.proxy_socket_address))??;
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
                            Ok(ProxyTcpConnection {
                                proxy_socket_address: self.proxy_socket_address,
                                frame_buffer_size: self.frame_buffer_size,
                                state: ProxyTcpConnectionRelayState {
                                    crypto_tcp_read_write: SinkWriter::new(StreamReader::new(
                                        CryptoLengthDelimitedFramed::new(
                                            io,
                                            self.state.proxy_encryption,
                                            self.state.agent_encryption,
                                            self.frame_buffer_size,
                                        ),
                                    )),
                                },
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
                    }
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
        .ok_or(CommonError::ConnectionExhausted(self.proxy_socket_address))??;
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
                self.proxy_socket_address
            ))),
        }
    }

    pub async fn close(&mut self) -> Result<(), CommonError> {
        self.state.tunnel_ctl_response_request_framed.close().await
    }
}

impl AsyncRead for ProxyTcpConnection<ProxyTcpConnectionRelayState> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let crypto_tcp_read_write = &mut self.get_mut().state.crypto_tcp_read_write;
        pin!(crypto_tcp_read_write);
        crypto_tcp_read_write.poll_read(cx, buf)
    }
}

impl AsyncWrite for ProxyTcpConnection<ProxyTcpConnectionRelayState> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let crypto_tcp_read_write = &mut self.get_mut().state.crypto_tcp_read_write;
        pin!(crypto_tcp_read_write);
        crypto_tcp_read_write.poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let crypto_tcp_read_write = &mut self.get_mut().state.crypto_tcp_read_write;
        pin!(crypto_tcp_read_write);
        crypto_tcp_read_write.poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let crypto_tcp_read_write = &mut self.get_mut().state.crypto_tcp_read_write;
        pin!(crypto_tcp_read_write);
        crypto_tcp_read_write.poll_shutdown(cx)
    }
}
