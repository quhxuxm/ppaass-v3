use crate::crypto::{decrypt_with_aes, encrypt_with_aes};
use crate::error::CommonError;
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use ppaass_protocol::Encryption;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{ready, Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::trace;
pub struct CryptoLengthDelimitedCodec<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    raw_length_delimited_framed: Framed<T, LengthDelimitedCodec>,
    decoder_encryption: Arc<Encryption>,
    encoder_encryption: Arc<Encryption>,
}

impl<T> CryptoLengthDelimitedCodec<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    pub fn new(
        tcp_stream: T,
        decoder_encryption: Encryption,
        encoder_encryption: Encryption,
        frame_buffer_size: usize,
    ) -> Self {
        Self {
            raw_length_delimited_framed: Framed::with_capacity(
                tcp_stream,
                LengthDelimitedCodec::new(),
                frame_buffer_size,
            ),
            decoder_encryption: Arc::new(decoder_encryption),
            encoder_encryption: Arc::new(encoder_encryption),
        }
    }
}

impl<T> Stream for CryptoLengthDelimitedCodec<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    type Item = Result<BytesMut, CommonError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let decoder_encryption = self.decoder_encryption.clone();
        let proxy_data = ready!(self
            .get_mut()
            .raw_length_delimited_framed
            .poll_next_unpin(cx));
        match proxy_data {
            None => Poll::Ready(None),
            Some(proxy_data) => match proxy_data {
                Err(e) => Poll::Ready(Some(Err(e.into()))),
                Ok(proxy_data) => match decoder_encryption.as_ref() {
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
impl<T> Sink<&[u8]> for CryptoLengthDelimitedCodec<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    type Error = CommonError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), CommonError>> {
        self.get_mut()
            .raw_length_delimited_framed
            .poll_ready_unpin(cx)
            .map_err(Into::into)
    }
    fn start_send(self: Pin<&mut Self>, item: &[u8]) -> Result<(), CommonError> {
        let item = match self.encoder_encryption.as_ref() {
            Encryption::Plain => BytesMut::from(item),
            Encryption::Aes(token) => BytesMut::from_iter(encrypt_with_aes(token, &item)?),
            Encryption::Blowfish(_) => {
                todo!()
            }
        };
        self.get_mut()
            .raw_length_delimited_framed
            .start_send_unpin(item.freeze())
            .map_err(Into::into)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), CommonError>> {
        self.get_mut()
            .raw_length_delimited_framed
            .poll_flush_unpin(cx)
            .map_err(Into::into)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), CommonError>> {
        self.get_mut()
            .raw_length_delimited_framed
            .poll_close_unpin(cx)
            .map_err(Into::into)
    }
}
