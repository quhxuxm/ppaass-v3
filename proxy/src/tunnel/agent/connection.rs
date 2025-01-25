use crate::error::ProxyError;
use crate::tunnel::agent::codec::HandshakeCodec;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{Sink, StreamExt};
use futures_util::{SinkExt, Stream};
use ppaass_common::crypto::{decrypt_with_aes, encrypt_with_aes, RsaCryptoRepository};
use ppaass_common::random_32_bytes;
use ppaass_protocol::{Encryption, HandshakeRequest, HandshakeResponse};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{ready, Context, Poll};
use tokio::io::AsyncWrite;
use tokio::net::TcpStream;
use tokio::pin;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Framed, FramedParts, LengthDelimitedCodec};
pub type AgentTcpConnectionWrite<R> = SplitSink<AgentTcpConnection<R>, BytesMut>;
pub type AgentTcpConnectionRead<R> = SplitStream<AgentTcpConnection<R>>;
pub enum AgentTcpConnection<R>
where
    R: RsaCryptoRepository + Sync + Send + 'static,
{
    New {
        agent_tcp_stream: Option<TcpStream>,
        agent_socket_address: SocketAddr,
        rsa_crypto_repo: Arc<R>,
    },
    Handshaked {
        agent_tcp_framed: Framed<TcpStream, LengthDelimitedCodec>,
        agent_socket_address: SocketAddr,
        authentication: String,
        agent_encryption: Encryption,
        server_encryption: Encryption,
    },
}

impl<R> AgentTcpConnection<R>
where
    R: RsaCryptoRepository + Sync + Send + 'static,
{
    pub fn new(
        agent_tcp_stream: TcpStream,
        agent_socket_address: SocketAddr,
        rsa_crypto_repo: Arc<R>,
    ) -> Self {
        Self::New {
            agent_tcp_stream: Some(agent_tcp_stream),
            agent_socket_address,
            rsa_crypto_repo,
        }
    }

    /// Handshake
    pub async fn handshake(&mut self) -> Result<(), ProxyError> {
        match self {
            AgentTcpConnection::Handshaked { .. } => Ok(()),
            AgentTcpConnection::New {
                agent_tcp_stream,
                agent_socket_address,
                rsa_crypto_repo,
            } => {
                let agent_tcp_stream = agent_tcp_stream.take().ok_or(ProxyError::Other(
                    format!("Fail to get agent tcp stream from object: {agent_socket_address}"),
                ))?;
                let mut handshake_framed = Framed::new(agent_tcp_stream, HandshakeCodec::new());
                let HandshakeRequest {
                    authentication,
                    encryption,
                } = handshake_framed
                    .next()
                    .await
                    .ok_or(ProxyError::AgentConnectionExhausted(*agent_socket_address))??;

                let rsa_crypto = rsa_crypto_repo
                    .get_rsa_crypto(&authentication)?
                    .ok_or(ProxyError::RsaCryptoNotFound(authentication.clone()))?;
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
                let raw_server_encryption_token = random_32_bytes();
                let encrypted_server_encryption_token =
                    rsa_crypto.encrypt(&raw_server_encryption_token)?;
                let server_encryption = Encryption::Aes(raw_server_encryption_token);
                let handshake_response = HandshakeResponse {
                    encryption: Encryption::Aes(encrypted_server_encryption_token),
                };
                handshake_framed.send(handshake_response).await?;
                let FramedParts {
                    io: agent_tcp_stream,
                    ..
                } = handshake_framed.into_parts();
                *self = AgentTcpConnection::Handshaked {
                    agent_tcp_framed: Framed::new(agent_tcp_stream, LengthDelimitedCodec::new()),
                    agent_socket_address: *agent_socket_address,
                    authentication,
                    agent_encryption,
                    server_encryption,
                };
                Ok(())
            }
        }
    }
}

impl<R> Stream for AgentTcpConnection<R>
where
    R: RsaCryptoRepository + Sync + Send + 'static,
{
    type Item = Result<BytesMut, ProxyError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            AgentTcpConnection::New { .. } => Poll::Pending,
            AgentTcpConnection::Handshaked {
                agent_tcp_framed,
                agent_encryption,
                ..
            } => {
                let agent_data = ready!(agent_tcp_framed.poll_next_unpin(cx));
                match agent_data {
                    None => Poll::Ready(None),
                    Some(agent_data) => match agent_data {
                        Err(e) => Poll::Ready(Some(Err(e.into()))),
                        Ok(agent_data) => match agent_encryption {
                            Encryption::Plain => Poll::Ready(Some(Ok(agent_data))),
                            Encryption::Aes(token) => match decrypt_with_aes(&token, &agent_data) {
                                Ok(raw_data) => {
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
    }
}

impl<R> Sink<BytesMut> for AgentTcpConnection<R>
where
    R: RsaCryptoRepository + Sync + Send + 'static,
{
    type Error = ProxyError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            AgentTcpConnection::New { .. } => Poll::Pending,
            AgentTcpConnection::Handshaked {
                agent_tcp_framed, ..
            } => agent_tcp_framed.poll_ready_unpin(cx).map_err(Into::into),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: BytesMut) -> Result<(), Self::Error> {
        match self.get_mut() {
            AgentTcpConnection::New {
                agent_socket_address,
                ..
            } => Err(ProxyError::Other(format!(
                "Agent connection still not handshake: {agent_socket_address}"
            ))),
            AgentTcpConnection::Handshaked {
                agent_tcp_framed,
                server_encryption,
                ..
            } => {
                let item = match server_encryption {
                    Encryption::Plain => item,
                    Encryption::Aes(token) => BytesMut::from_iter(encrypt_with_aes(token, &item)?),
                    Encryption::Blowfish(_) => {
                        todo!()
                    }
                };
                agent_tcp_framed
                    .start_send_unpin(item.freeze())
                    .map_err(Into::into)
            }
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            AgentTcpConnection::New { .. } => Poll::Pending,
            AgentTcpConnection::Handshaked {
                agent_tcp_framed, ..
            } => agent_tcp_framed.poll_flush_unpin(cx).map_err(Into::into),
        }
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            AgentTcpConnection::New {
                agent_tcp_stream, ..
            } => match agent_tcp_stream {
                None => Poll::Ready(Ok(())),
                Some(agent_tcp_stream) => {
                    pin!(agent_tcp_stream);
                    agent_tcp_stream.poll_shutdown(cx).map_err(Into::into)
                }
            },
            AgentTcpConnection::Handshaked {
                agent_tcp_framed, ..
            } => agent_tcp_framed.poll_close_unpin(cx).map_err(Into::into),
        }
    }
}
