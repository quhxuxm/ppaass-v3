use crate::error::ProxyError;
use ppaass_protocol::{HandshakeRequest, HandshakeResponse};
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};
pub struct HandshakeCodec;

impl Decoder for HandshakeCodec {
    type Item = HandshakeRequest;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        todo!()
    }
}

impl Encoder<HandshakeResponse> for HandshakeCodec {
    type Error = ProxyError;
    fn encode(&mut self, item: HandshakeResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        todo!()
    }
}
