use futures_util::stream::SplitSink;
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use ppaass_common::error::CommonError;
use ppaass_protocol::UnifiedAddress;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{BytesCodec, Framed};
use tracing::debug;
pub type DestinationTcpEndpointWrite = SplitSink<DestinationTcpEndpoint, BytesMut>;
pub struct DestinationTcpEndpoint {
    destination_tcp_framed: Framed<TcpStream, BytesCodec>,
    destination_address: UnifiedAddress,
}

impl DestinationTcpEndpoint {
    pub async fn connect(destination_address: UnifiedAddress) -> Result<Self, CommonError> {
        let destination_socks_addrs: Vec<SocketAddr> =
            destination_address.clone().try_into().map_err(|e| {
                CommonError::Other(format!(
                    "Fail to convert destination address to socket address: {e}"
                ))
            })?;
        let destination_tcp_stream = TcpStream::connect(destination_socks_addrs.as_slice()).await?;
        debug!("Connected to destination success: {}", destination_address);
        Ok(DestinationTcpEndpoint {
            destination_address,
            destination_tcp_framed: Framed::new(destination_tcp_stream, BytesCodec::new()),
        })
    }
}

impl Stream for DestinationTcpEndpoint {
    type Item = Result<BytesMut, CommonError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut()
            .destination_tcp_framed
            .poll_next_unpin(cx)
            .map_err(Into::into)
    }
}

impl Sink<BytesMut> for DestinationTcpEndpoint {
    type Error = CommonError;
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
