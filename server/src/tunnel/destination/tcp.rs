use crate::error::ServerError;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use ppaass_protocol::UnifiedAddress;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{BytesCodec, Framed};
pub type DestinationTcpEndpointRead = SplitStream<DestinationTcpEndpoint>;
pub type DestinationTcpEndpointWrite = SplitSink<DestinationTcpEndpoint, BytesMut>;
pub struct DestinationTcpEndpoint {
    destination_tcp_framed: Framed<TcpStream, BytesCodec>,
    destination_address: UnifiedAddress,
}

impl DestinationTcpEndpoint {
    pub async fn connect(destination_address: UnifiedAddress) -> Result<Self, ServerError> {
        let destination_socks_addrs: Vec<SocketAddr> = destination_address.clone().try_into()?;
        let destination_tcp_stream = TcpStream::connect(destination_socks_addrs.as_slice()).await?;
        Ok(DestinationTcpEndpoint {
            destination_address,
            destination_tcp_framed: Framed::new(destination_tcp_stream, BytesCodec::new()),
        })
    }
}

impl Stream for DestinationTcpEndpoint {
    type Item = Result<BytesMut, ServerError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut()
            .destination_tcp_framed
            .poll_next_unpin(cx)
            .map_err(Into::into)
    }
}

impl Sink<BytesMut> for DestinationTcpEndpoint {
    type Error = ServerError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        SinkExt::<BytesMut>::poll_ready_unpin(&mut self.get_mut().destination_tcp_framed, cx)
            .map_err(Into::into)
    }
    fn start_send(self: Pin<&mut Self>, item: BytesMut) -> Result<(), Self::Error> {
        self.get_mut()
            .destination_tcp_framed
            .start_send_unpin(item)
            .map_err(Into::into)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        SinkExt::<BytesMut>::poll_flush_unpin(&mut self.get_mut().destination_tcp_framed, cx)
            .map_err(Into::into)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        SinkExt::<BytesMut>::poll_close_unpin(&mut self.get_mut().destination_tcp_framed, cx)
            .map_err(Into::into)
    }
}
