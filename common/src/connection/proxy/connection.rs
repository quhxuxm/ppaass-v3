use crate::connection::codec::{HandshakeRequestEncoder, HandshakeResponseDecoder};

use crate::connection::CryptoLengthDelimitedFramed;
use crate::crypto::RsaCryptoRepository;
use crate::error::CommonError;
use crate::random_32_bytes;
use futures_util::{SinkExt, StreamExt};
use ppaass_protocol::{
    Encryption, HandshakeRequest, HandshakeResponse, HeartbeatRequest, TunnelControlRequest,
    TunnelControlResponse, TunnelInitFailureReason, TunnelInitRequest, TunnelInitResponse,
};
use std::fmt::{Debug, Formatter};
use std::io::Error;
use std::net::SocketAddr;
use std::pin::Pin;
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
    crypto_tcp_framed: CryptoLengthDelimitedFramed<TfoStream>,
}

pub struct ProxyTcpConnectionRelayState {
    crypto_tcp_read_write:
        SinkWriter<StreamReader<CryptoLengthDelimitedFramed<TfoStream>, BytesMut>>,
}
#[derive(Debug, Clone)]
pub struct ProxyTcpConnectionInfo {
    proxy_addresses: Vec<SocketAddr>,
    authentication: String,
    frame_buffer_size: usize,
    connect_timeout: u64,
}
impl ProxyTcpConnectionInfo {
    pub fn new(
        proxy_addresses: Vec<SocketAddr>,
        authentication: String,
        frame_buffer_size: usize,
        connect_timeout: u64,
    ) -> Self {
        Self {
            proxy_addresses,
            authentication,
            frame_buffer_size,
            connect_timeout,
        }
    }
    pub fn authentication(&self) -> &str {
        &self.authentication
    }
    pub fn proxy_addresses(&self) -> &[SocketAddr] {
        &self.proxy_addresses
    }
    pub fn frame_buffer_size(&self) -> usize {
        self.frame_buffer_size
    }
    pub fn connect_timeout(&self) -> u64 {
        self.connect_timeout
    }
}
pub struct ProxyTcpConnection<T> {
    proxy_socket_address: SocketAddr,
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
    pub async fn create<R>(
        proxy_tcp_connection_info: ProxyTcpConnectionInfo,
        rsa_crypto_repo: &R,
    ) -> Result<ProxyTcpConnection<ProxyTcpConnectionTunnelCtlState>, CommonError>
    where
        R: RsaCryptoRepository + Sync + Send + 'static,
    {
        let proxy_tcp_stream = timeout(
            Duration::from_secs(proxy_tcp_connection_info.connect_timeout()),
            TfoStream::connect(proxy_tcp_connection_info.proxy_addresses()[0]),
        )
        .await??;
        proxy_tcp_stream.set_nodelay(true)?;
        proxy_tcp_stream.set_linger(None)?;
        let proxy_socket_address = proxy_tcp_stream.peer_addr()?;
        let agent_encryption_raw_aes_token = random_32_bytes();
        let rsa_crypto = rsa_crypto_repo
            .get_rsa_crypto(proxy_tcp_connection_info.authentication())?
            .ok_or(CommonError::RsaCryptoNotFound(
                proxy_tcp_connection_info.authentication().to_owned(),
            ))?;
        let encrypt_agent_encryption_aes_token =
            rsa_crypto.encrypt(&agent_encryption_raw_aes_token)?;
        let encrypt_agent_encryption = Encryption::Aes(encrypt_agent_encryption_aes_token);
        let mut handshake_request_framed =
            Framed::new(proxy_tcp_stream, HandshakeRequestEncoder::new());
        let handshake_request = HandshakeRequest {
            authentication: proxy_tcp_connection_info.authentication().to_owned(),
            encryption: encrypt_agent_encryption,
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
        let agent_encryption = Encryption::Aes(agent_encryption_raw_aes_token);
        let proxy_encryption = match proxy_encryption {
            Encryption::Plain => proxy_encryption,
            Encryption::Aes(encrypted_token) => {
                Encryption::Aes(rsa_crypto.decrypt(&encrypted_token)?)
            }
            Encryption::Blowfish(encrypted_token) => {
                Encryption::Blowfish(rsa_crypto.encrypt(&encrypted_token)?)
            }
        };
        let FramedParts {
            io: proxy_tcp_stream,
            ..
        } = handshake_response_framed.into_parts();
        let proxy_socket_address = proxy_tcp_stream.peer_addr()?;
        Ok(ProxyTcpConnection {
            state: ProxyTcpConnectionTunnelCtlState {
                crypto_tcp_framed: CryptoLengthDelimitedFramed::new(
                    proxy_tcp_stream,
                    proxy_encryption,
                    agent_encryption,
                    proxy_tcp_connection_info.frame_buffer_size(),
                ),
            },
            proxy_socket_address,
        })
    }
}
impl ProxyTcpConnection<ProxyTcpConnectionTunnelCtlState> {
    pub async fn tunnel_init(
        self,
        tunnel_init_request: TunnelInitRequest,
    ) -> Result<ProxyTcpConnection<ProxyTcpConnectionRelayState>, CommonError> {
        let tunnel_ctl_request = TunnelControlRequest::TunnelInit(tunnel_init_request);
        let raw_tunnel_ctl_request_bytes = bincode::serialize(&tunnel_ctl_request)?;
        let mut crypto_tcp_framed = self.state.crypto_tcp_framed;
        crypto_tcp_framed
            .send(&raw_tunnel_ctl_request_bytes)
            .await?;
        loop {
            let tunnel_ctl_response_bytes = crypto_tcp_framed
                .next()
                .await
                .ok_or(CommonError::ConnectionExhausted(self.proxy_socket_address))??;
            let tunnel_ctl_response: TunnelControlResponse =
                bincode::deserialize(&tunnel_ctl_response_bytes)?;
            match tunnel_ctl_response {
                TunnelControlResponse::Heartbeat(heart_beat) => {
                    debug!("Receive heartbeat response from proxy connection: {heart_beat:?}");
                    continue;
                }
                TunnelControlResponse::TunnelInit(tunnel_init_response) => {
                    return match tunnel_init_response {
                        TunnelInitResponse::Success => Ok(ProxyTcpConnection {
                            proxy_socket_address: self.proxy_socket_address,
                            state: ProxyTcpConnectionRelayState {
                                crypto_tcp_read_write: SinkWriter::new(StreamReader::new(
                                    crypto_tcp_framed,
                                )),
                            },
                        }),
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
    pub async fn heartbeat(&mut self, timeout_seconds: u64) -> Result<(), CommonError> {
        let heartbeat_request = TunnelControlRequest::Heartbeat(HeartbeatRequest::new());
        let raw_tunnel_ctl_request = bincode::serialize(&heartbeat_request)?;
        self.state
            .crypto_tcp_framed
            .send(&raw_tunnel_ctl_request)
            .await?;
        let raw_tunnel_ctl_response_bytes = timeout(
            Duration::from_secs(timeout_seconds),
            self.state.crypto_tcp_framed.next(),
        )
        .await?
        .ok_or(CommonError::ConnectionExhausted(self.proxy_socket_address))??;
        let tunnel_ctl_response: TunnelControlResponse =
            bincode::deserialize(&raw_tunnel_ctl_response_bytes)?;
        match tunnel_ctl_response {
            TunnelControlResponse::Heartbeat(heartbeat_response) => {
                debug!("Receive heartbeat response from proxy connection: {heartbeat_response:?}");
                Ok(())
            }
            TunnelControlResponse::TunnelInit(_) => Err(CommonError::Other(format!(
                "Receive tunnel init response from proxy connection: {}",
                self.proxy_socket_address
            ))),
        }
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
