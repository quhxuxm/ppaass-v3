use futures_util::{Sink, SinkExt, Stream, StreamExt};
use ppaass_common::error::CommonError;

use ppaass_common::UnifiedAddress;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::timeout;
use tokio_tfo::TfoStream;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{BytesCodec, Framed};
use tracing::debug;

pub struct DestinationTcpEndpoint {
    destination_tcp_framed: Framed<TfoStream, BytesCodec>,
    destination_address: UnifiedAddress,
}

impl DestinationTcpEndpoint {
    pub async fn connect(
        destination_address: UnifiedAddress,
        _keep_alive: bool,
        connect_timeout: u64,
    ) -> Result<Self, CommonError> {
        let destination_socks_addrs: Vec<SocketAddr> =
            destination_address.clone().try_into().map_err(|e| {
                CommonError::Other(format!(
                    "Fail to convert destination address to socket address: {e}"
                ))
            })?;
        let destination_tcp_stream = timeout(
            Duration::from_secs(connect_timeout),
            TfoStream::connect(destination_socks_addrs.as_slice()[0]),
        )
        .await??;
        destination_tcp_stream.set_nodelay(true)?;
        debug!("Connected to destination success: {}", destination_address);
        Ok(DestinationTcpEndpoint {
            destination_tcp_framed: Framed::new(destination_tcp_stream, BytesCodec::new()),
            destination_address,
        })
    }

    pub fn destination_address(&self) -> &UnifiedAddress {
        &self.destination_address
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

impl Sink<&[u8]> for DestinationTcpEndpoint {
    type Error = CommonError;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        SinkExt::<BytesMut>::poll_ready_unpin(&mut self.get_mut().destination_tcp_framed, cx)
            .map_err(Into::into)
    }
    fn start_send(self: Pin<&mut Self>, item: &[u8]) -> Result<(), Self::Error> {
        self.get_mut()
            .destination_tcp_framed
            .start_send_unpin(BytesMut::from(item))
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
