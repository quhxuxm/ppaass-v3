use crate::connection::codec::{HandshakeRequestDecoder, HandshakeResponseEncoder};
use crate::connection::CryptoLengthDelimitedFramed;
use crate::crypto::RsaCryptoRepository;
use crate::error::CommonError;
use crate::random_32_bytes;
use futures_util::{Sink, StreamExt};
use futures_util::{SinkExt, Stream};
use ppaass_protocol::{
    Encryption, HandshakeRequest, HandshakeResponse, HeartbeatResponse, TunnelControlRequest,
    TunnelControlResponse, TunnelInitRequest, TunnelInitResponse,
};
use std::io::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::pin;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Framed, FramedParts};
use tokio_util::io::{SinkWriter, StreamReader};
use tracing::debug;
pub struct AgentTcpConnectionNewState {}
pub struct AgentTcpConnectionTunnelCtlState<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    crypto_tcp_framed: CryptoLengthDelimitedFramed<T>,
}
pub struct AgentTcpConnectionTcpRelayState<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    crypto_tcp_read_write: SinkWriter<StreamReader<CryptoLengthDelimitedFramed<T>, BytesMut>>,
}

pub struct AgentTcpConnectionUdpRelayState<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    crypto_tcp_framed: CryptoLengthDelimitedFramed<T>,
}
pub struct AgentTcpConnection<S> {
    agent_socket_address: SocketAddr,
    authentication: String,
    state: S,
}
impl<S> AgentTcpConnection<S> {
    pub fn agent_socket_address(&self) -> SocketAddr {
        self.agent_socket_address
    }
}
impl AgentTcpConnection<AgentTcpConnectionNewState> {
    pub async fn create<T, R>(
        agent_tcp_stream: T,
        agent_socket_address: SocketAddr,
        rsa_crypto_repo: &R,
        frame_buffer_size: usize,
    ) -> Result<AgentTcpConnection<AgentTcpConnectionTunnelCtlState<T>>, CommonError>
    where
        T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
        R: RsaCryptoRepository + Sync + Send + 'static,
    {
        let mut handshake_request_framed =
            Framed::new(agent_tcp_stream, HandshakeRequestDecoder::new());
        let HandshakeRequest {
            authentication,
            encryption,
        } = handshake_request_framed
            .next()
            .await
            .ok_or(CommonError::ConnectionExhausted(agent_socket_address))??;
        let rsa_crypto = rsa_crypto_repo
            .get_rsa_crypto(&authentication)?
            .ok_or(CommonError::RsaCryptoNotFound(authentication.clone()))?;
        let agent_encryption = match encryption {
            Encryption::Plain => encryption,
            Encryption::Aes(token) => {
                let decrypted_token = rsa_crypto.decrypt(&token)?;
                Encryption::Aes(decrypted_token)
            }
            Encryption::Blowfish(token) => {
                let decrypted_token = rsa_crypto.decrypt(&token)?;
                Encryption::Blowfish(decrypted_token)
            }
        };
        let raw_proxy_encryption_token = random_32_bytes();
        let encrypted_proxy_encryption_token = rsa_crypto.encrypt(&raw_proxy_encryption_token)?;
        let proxy_encryption = Encryption::Aes(raw_proxy_encryption_token);
        let handshake_response = HandshakeResponse {
            encryption: Encryption::Aes(encrypted_proxy_encryption_token),
        };
        let FramedParts {
            io: agent_tcp_stream,
            ..
        } = handshake_request_framed.into_parts();
        let mut handshake_response_framed =
            Framed::new(agent_tcp_stream, HandshakeResponseEncoder::new());
        handshake_response_framed.send(handshake_response).await?;
        let FramedParts {
            io: agent_tcp_stream,
            ..
        } = handshake_response_framed.into_parts();
        Ok(AgentTcpConnection {
            agent_socket_address,
            authentication,
            state: AgentTcpConnectionTunnelCtlState {
                crypto_tcp_framed: CryptoLengthDelimitedFramed::new(
                    agent_tcp_stream,
                    agent_encryption,
                    proxy_encryption,
                    frame_buffer_size,
                ),
            },
        })
    }
}
impl<T> AgentTcpConnection<AgentTcpConnectionTunnelCtlState<T>>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    pub async fn wait_tunnel_init(&mut self) -> Result<TunnelInitRequest, CommonError> {
        loop {
            let tunnel_ctl_request_bytes = self
                .state
                .crypto_tcp_framed
                .next()
                .await
                .ok_or(CommonError::ConnectionExhausted(self.agent_socket_address))??;
            let tunnel_ctl_request: TunnelControlRequest =
                bincode::deserialize(&tunnel_ctl_request_bytes)?;
            match tunnel_ctl_request {
                TunnelControlRequest::Heartbeat(heartbeat_request) => {
                    debug!("Receive heartbeat request from agent connection [{}]: {heartbeat_request:?}", self.agent_socket_address);
                    let heartbeat_response =
                        TunnelControlResponse::Heartbeat(HeartbeatResponse::new());
                    let heartbeat_response_bytes = bincode::serialize(&heartbeat_response)?;
                    self.state
                        .crypto_tcp_framed
                        .send(&heartbeat_response_bytes)
                        .await?;
                    continue;
                }
                TunnelControlRequest::TunnelInit(tunnel_init_request) => {
                    return Ok(tunnel_init_request)
                }
            }
        }
    }

    pub async fn response_tcp_tunnel_init(
        mut self,
        tunnel_init_response: TunnelInitResponse,
    ) -> Result<AgentTcpConnection<AgentTcpConnectionTcpRelayState<T>>, CommonError> {
        let tunnel_ctl_response = TunnelControlResponse::TunnelInit(tunnel_init_response);
        let tunnel_ctl_response_bytes = bincode::serialize(&tunnel_ctl_response)?;
        self.state
            .crypto_tcp_framed
            .send(&tunnel_ctl_response_bytes)
            .await?;
        Ok(AgentTcpConnection {
            agent_socket_address: self.agent_socket_address,
            authentication: self.authentication,
            state: AgentTcpConnectionTcpRelayState {
                crypto_tcp_read_write: SinkWriter::new(StreamReader::new(
                    self.state.crypto_tcp_framed,
                )),
            },
        })
    }

    pub async fn response_udp_tunnel_init(
        mut self,
        tunnel_init_response: TunnelInitResponse,
    ) -> Result<AgentTcpConnection<AgentTcpConnectionUdpRelayState<T>>, CommonError> {
        let tunnel_ctl_response = TunnelControlResponse::TunnelInit(tunnel_init_response);
        let tunnel_ctl_response_bytes = bincode::serialize(&tunnel_ctl_response)?;
        self.state
            .crypto_tcp_framed
            .send(&tunnel_ctl_response_bytes)
            .await?;
        Ok(AgentTcpConnection {
            agent_socket_address: self.agent_socket_address,
            authentication: self.authentication,
            state: AgentTcpConnectionUdpRelayState {
                crypto_tcp_framed: self.state.crypto_tcp_framed,
            },
        })
    }
}
impl<T> AsyncRead for AgentTcpConnection<AgentTcpConnectionTcpRelayState<T>>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
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
impl<T> AsyncWrite for AgentTcpConnection<AgentTcpConnectionTcpRelayState<T>>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
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
impl<T> Stream for AgentTcpConnection<AgentTcpConnectionUdpRelayState<T>>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    type Item = Result<BytesMut, CommonError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let crypto_tcp_framed = &mut self.get_mut().state.crypto_tcp_framed;
        pin!(crypto_tcp_framed);
        crypto_tcp_framed.poll_next(cx)
    }
}
impl<T> Sink<BytesMut> for AgentTcpConnection<AgentTcpConnectionUdpRelayState<T>>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    type Error = CommonError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let crypto_tcp_framed = &mut self.get_mut().state.crypto_tcp_framed;
        pin!(crypto_tcp_framed);
        crypto_tcp_framed.poll_ready(cx)
    }
    fn start_send(self: Pin<&mut Self>, item: BytesMut) -> Result<(), Self::Error> {
        let crypto_tcp_framed = &mut self.get_mut().state.crypto_tcp_framed;
        pin!(crypto_tcp_framed);
        crypto_tcp_framed.start_send(&item)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let crypto_tcp_framed = &mut self.get_mut().state.crypto_tcp_framed;
        pin!(crypto_tcp_framed);
        crypto_tcp_framed.poll_flush(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let crypto_tcp_framed = &mut self.get_mut().state.crypto_tcp_framed;
        pin!(crypto_tcp_framed);
        crypto_tcp_framed.poll_close(cx)
    }
}
