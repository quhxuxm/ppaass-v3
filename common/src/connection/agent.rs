use crate::connection::codec::{HandshakeRequestDecoder, HandshakeResponseEncoder};
use crate::crypto::{decrypt_with_aes, encrypt_with_aes, RsaCryptoRepository};
use crate::error::CommonError;
use crate::random_32_bytes;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{Sink, StreamExt};
use futures_util::{SinkExt, Stream};
use ppaass_protocol::{Encryption, HandshakeRequest, HandshakeResponse};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{ready, Context, Poll};
use tokio::net::TcpStream;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Framed, FramedParts, LengthDelimitedCodec};
pub type AgentTcpConnectionWrite = SplitSink<AgentTcpConnection, BytesMut>;
pub type AgentTcpConnectionRead = SplitStream<AgentTcpConnection>;
pub struct AgentTcpConnection {
    agent_tcp_framed: Framed<TcpStream, LengthDelimitedCodec>,
    agent_socket_address: SocketAddr,
    authentication: String,
    agent_encryption: Arc<Encryption>,
    proxy_encryption: Arc<Encryption>,
}

impl AgentTcpConnection {
    pub async fn create<R>(
        agent_tcp_stream: TcpStream,
        agent_socket_address: SocketAddr,
        rsa_crypto_repo: &R,
    ) -> Result<Self, CommonError>
    where
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

        Ok(Self {
            agent_tcp_framed: Framed::new(agent_tcp_stream, LengthDelimitedCodec::new()),
            agent_socket_address,
            authentication,
            agent_encryption: Arc::new(agent_encryption),
            proxy_encryption: Arc::new(proxy_encryption),
        })
    }
}

impl Stream for AgentTcpConnection {
    type Item = Result<BytesMut, CommonError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let agent_encryption = self.agent_encryption.clone();
        let agent_data = ready!(self.get_mut().agent_tcp_framed.poll_next_unpin(cx));
        match agent_data {
            None => Poll::Ready(None),
            Some(agent_data) => match agent_data {
                Err(e) => Poll::Ready(Some(Err(e.into()))),
                Ok(agent_data) => match agent_encryption.as_ref() {
                    Encryption::Plain => Poll::Ready(Some(Ok(agent_data))),
                    Encryption::Aes(token) => match decrypt_with_aes(&token, &agent_data) {
                        Ok(raw_data) => Poll::Ready(Some(Ok(BytesMut::from_iter(raw_data)))),
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

impl Sink<BytesMut> for AgentTcpConnection {
    type Error = CommonError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .agent_tcp_framed
            .poll_ready_unpin(cx)
            .map_err(Into::into)
    }

    fn start_send(self: Pin<&mut Self>, item: BytesMut) -> Result<(), Self::Error> {
        let item = match self.proxy_encryption.as_ref() {
            Encryption::Plain => item,
            Encryption::Aes(token) => BytesMut::from_iter(encrypt_with_aes(token, &item)?),
            Encryption::Blowfish(_) => {
                todo!()
            }
        };
        self.get_mut()
            .agent_tcp_framed
            .start_send_unpin(item.freeze())
            .map_err(Into::into)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .agent_tcp_framed
            .poll_flush_unpin(cx)
            .map_err(Into::into)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .agent_tcp_framed
            .poll_close_unpin(cx)
            .map_err(Into::into)
    }
}
