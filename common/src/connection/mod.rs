mod agent;
mod codec;
mod proxy;
use crate::connection::codec::CryptoLengthDelimitedCodec;

use crate::error::CommonError;
pub use agent::*;
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use ppaass_protocol::Encryption;
pub use proxy::*;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Framed;

struct CryptoLengthDelimitedFramed<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    crypto_length_delimited_framed: Framed<T, CryptoLengthDelimitedCodec>,
}

impl<T> CryptoLengthDelimitedFramed<T>
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
            crypto_length_delimited_framed: Framed::with_capacity(
                tcp_stream,
                CryptoLengthDelimitedCodec::new(
                    Arc::new(decoder_encryption),
                    Arc::new(encoder_encryption),
                ),
                frame_buffer_size,
            ),
        }
    }
}

impl<T> Stream for CryptoLengthDelimitedFramed<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    type Item = Result<BytesMut, CommonError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut()
            .crypto_length_delimited_framed
            .poll_next_unpin(cx)
    }
}
impl<T> Sink<&[u8]> for CryptoLengthDelimitedFramed<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    type Error = CommonError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), CommonError>> {
        self.get_mut()
            .crypto_length_delimited_framed
            .poll_ready_unpin(cx)
    }
    fn start_send(self: Pin<&mut Self>, item: &[u8]) -> Result<(), CommonError> {
        self.get_mut()
            .crypto_length_delimited_framed
            .start_send_unpin(item.to_vec().into())
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), CommonError>> {
        self.get_mut()
            .crypto_length_delimited_framed
            .poll_flush_unpin(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), CommonError>> {
        self.get_mut()
            .crypto_length_delimited_framed
            .poll_close_unpin(cx)
    }
}
