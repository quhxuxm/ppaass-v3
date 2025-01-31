use crate::connection::codec::{HandshakeRequestEncoder, HandshakeResponseDecoder};
use crate::crypto::{decrypt_with_aes, encrypt_with_aes, RsaCryptoRepository};
use crate::error::CommonError;
use crate::random_32_bytes;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use ppaass_protocol::{Encryption, HandshakeRequest, HandshakeResponse};
use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{ready, Context, Poll};
use tokio::net::TcpStream;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Framed, FramedParts, LengthDelimitedCodec};
use tracing::{debug, trace};
pub type ProxyTcpConnectionWrite = SplitSink<ProxyTcpConnection, BytesMut>;
pub type ProxyTcpConnectionRead = SplitStream<ProxyTcpConnection>;

#[derive(Debug, Clone)]
pub struct ProxyTcpConnectionInfo {
    proxy_addresses: Vec<SocketAddr>,
    authentication: String,
}

impl ProxyTcpConnectionInfo {
    pub fn new(proxy_addresses: Vec<SocketAddr>, authentication: String) -> Self {
        Self {
            proxy_addresses,
            authentication,
        }
    }
    pub fn authentication(&self) -> &str {
        &self.authentication
    }
    pub fn proxy_addresses(&self) -> &[SocketAddr] {
        &self.proxy_addresses
    }
}

pub struct ProxyTcpConnection {
    proxy_tcp_framed: Framed<TcpStream, LengthDelimitedCodec>,
    agent_encryption: Arc<Encryption>,
    proxy_encryption: Arc<Encryption>,
    proxy_socket_address: SocketAddr,
}
impl Debug for ProxyTcpConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProxyTcpConnection: {}", self.proxy_socket_address)
    }
}
impl ProxyTcpConnection {
    pub async fn create<R>(
        proxy_tcp_connection_info: ProxyTcpConnectionInfo,
        rsa_crypto_repo: &R,
    ) -> Result<Self, CommonError>
    where
        R: RsaCryptoRepository + Sync + Send + 'static,
    {
        let proxy_tcp_stream =
            TcpStream::connect(proxy_tcp_connection_info.proxy_addresses()).await?;
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
        Ok(Self {
            proxy_tcp_framed: Framed::new(proxy_tcp_stream, LengthDelimitedCodec::new()),
            agent_encryption: Arc::new(agent_encryption),
            proxy_encryption: Arc::new(proxy_encryption),
            proxy_socket_address,
        })
    }

    pub fn proxy_socket_address(&self) -> SocketAddr {
        self.proxy_socket_address
    }
}

impl Stream for ProxyTcpConnection {
    type Item = Result<BytesMut, CommonError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let proxy_encryption = self.proxy_encryption.clone();
        let proxy_data = ready!(self.get_mut().proxy_tcp_framed.poll_next_unpin(cx));
        match proxy_data {
            None => Poll::Ready(None),
            Some(proxy_data) => match proxy_data {
                Err(e) => Poll::Ready(Some(Err(e.into()))),
                Ok(proxy_data) => match proxy_encryption.as_ref() {
                    Encryption::Plain => Poll::Ready(Some(Ok(proxy_data))),
                    Encryption::Aes(token) => match decrypt_with_aes(&token, &proxy_data) {
                        Ok(raw_data) => {
                            trace!(
                                "Proxy tcp connection receive data:\n{}",
                                pretty_hex::pretty_hex(&raw_data)
                            );
                            Poll::Ready(Some(Ok(BytesMut::from_iter(raw_data))))
                        }
                        Err(e) => Poll::Ready(Some(Err(e.into()))),
                    },
                    Encryption::Blowfish(_) => {
                        todo!()
                    }
                },
            },
        }
    }
}

impl Sink<&[u8]> for ProxyTcpConnection {
    type Error = CommonError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), CommonError>> {
        self.get_mut()
            .proxy_tcp_framed
            .poll_ready_unpin(cx)
            .map_err(Into::into)
    }

    fn start_send(self: Pin<&mut Self>, item: &[u8]) -> Result<(), CommonError> {
        let item = match self.agent_encryption.as_ref() {
            Encryption::Plain => BytesMut::from(item),
            Encryption::Aes(token) => {
                trace!(
                    "Proxy tcp connection send data:\n{}",
                    pretty_hex::pretty_hex(&item)
                );
                BytesMut::from_iter(encrypt_with_aes(token, &item)?)
            }
            Encryption::Blowfish(_) => {
                todo!()
            }
        };
        self.get_mut()
            .proxy_tcp_framed
            .start_send_unpin(item.freeze())
            .map_err(Into::into)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), CommonError>> {
        self.get_mut()
            .proxy_tcp_framed
            .poll_flush_unpin(cx)
            .map_err(Into::into)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), CommonError>> {
        self.get_mut()
            .proxy_tcp_framed
            .poll_close_unpin(cx)
            .map_err(Into::into)
    }
}
