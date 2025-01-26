use crate::error::CommonError;
use ppaass_protocol::HandshakeResponse;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
pub struct HandshakeResponseDecoder {
    length_delimited_codec: LengthDelimitedCodec,
}

impl HandshakeResponseDecoder {
    pub fn new() -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
        }
    }
}

impl Decoder for HandshakeResponseDecoder {
    type Item = HandshakeResponse;
    type Error = CommonError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let raw_bytes = self.length_delimited_codec.decode(src)?;
        match raw_bytes {
            None => Ok(None),
            Some(raw_bytes) => {
                let handshake = bincode::deserialize::<HandshakeResponse>(&raw_bytes)?;
                Ok(Some(handshake))
            }
        }
    }
}

pub struct HandshakeResponseEncoder {
    length_delimited_codec: LengthDelimitedCodec,
}
impl HandshakeResponseEncoder {
    pub fn new() -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
        }
    }
}

impl Encoder<HandshakeResponse> for HandshakeResponseEncoder {
    type Error = CommonError;
    fn encode(&mut self, item: HandshakeResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let raw_bytes = bincode::serialize(&item)?;
        self.length_delimited_codec
            .encode(raw_bytes.into(), dst)
            .map_err(Into::into)
    }
}
