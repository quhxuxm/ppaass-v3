use crate::error::ProxyError;
use ppaass_protocol::{HandshakeRequest, HandshakeResponse};
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
pub struct HandshakeCodec {
    length_delimited_codec: LengthDelimitedCodec,
}

impl HandshakeCodec {
    pub fn new() -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
        }
    }
}

impl Decoder for HandshakeCodec {
    type Item = HandshakeRequest;
    type Error = ProxyError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let raw_bytes = self.length_delimited_codec.decode(src)?;
        match raw_bytes {
            None => Ok(None),
            Some(raw_bytes) => {
                let handshake = bincode::deserialize::<HandshakeRequest>(&raw_bytes)?;
                Ok(Some(handshake))
            }
        }
    }
}

impl Encoder<HandshakeResponse> for HandshakeCodec {
    type Error = ProxyError;
    fn encode(&mut self, item: HandshakeResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let raw_bytes = bincode::serialize(&item)?;
        self.length_delimited_codec
            .encode(raw_bytes.into(), dst)
            .map_err(Into::into)
    }
}
