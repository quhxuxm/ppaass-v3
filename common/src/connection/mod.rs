mod agent;
mod codec;
mod proxy;
use crate::connection::codec::CryptoLengthDelimitedCodec;
use crate::error::CommonError;
pub use agent::*;
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use ppaass_protocol::Encryption;
pub use proxy::*;
use std::io::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio::pin;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Framed;
use tokio_util::io::{SinkWriter, StreamReader};
pub struct CryptoLengthDelimitedFramed<T>
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
        decoder_encryption: Arc<Encryption>,
        encoder_encryption: Arc<Encryption>,
        frame_buffer_size: usize,
    ) -> Self {
        Self {
            crypto_length_delimited_framed: Framed::with_capacity(
                tcp_stream,
                CryptoLengthDelimitedCodec::new(decoder_encryption, encoder_encryption),
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
            .start_send_unpin(BytesMut::from(item))
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

pub struct FramedConnection<S> {
    state: S,
    socket_address: SocketAddr,
    frame_buffer_size: usize,
}

impl<S> FramedConnection<S> {
    pub fn new(state: S, socket_address: SocketAddr, frame_buffer_size: usize) -> Self {
        Self {
            state,
            socket_address,
            frame_buffer_size,
        }
    }
}

impl AsyncRead
    for FramedConnection<SinkWriter<StreamReader<CryptoLengthDelimitedFramed<TcpStream>, BytesMut>>>
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let crypto_tcp_read_write = &mut self.get_mut().state;
        pin!(crypto_tcp_read_write);
        crypto_tcp_read_write.poll_read(cx, buf)
    }
}

impl AsyncWrite
    for FramedConnection<SinkWriter<StreamReader<CryptoLengthDelimitedFramed<TcpStream>, BytesMut>>>
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let crypto_tcp_read_write = &mut self.get_mut().state;
        pin!(crypto_tcp_read_write);
        crypto_tcp_read_write.poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let crypto_tcp_read_write = &mut self.get_mut().state;
        pin!(crypto_tcp_read_write);
        crypto_tcp_read_write.poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let crypto_tcp_read_write = &mut self.get_mut().state;
        pin!(crypto_tcp_read_write);
        crypto_tcp_read_write.poll_shutdown(cx)
    }
}
