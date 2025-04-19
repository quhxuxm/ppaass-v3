use crate::error::CommonError;
use ppaass_protocol::HandshakeRequest;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
pub struct HandshakeRequestDecoder {
    length_delimited_codec: LengthDelimitedCodec,
}

impl HandshakeRequestDecoder {
    pub fn new() -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
        }
    }
}

impl Decoder for HandshakeRequestDecoder {
    type Item = HandshakeRequest;
    type Error = CommonError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let raw_bytes = self.length_delimited_codec.decode(src)?;
        match raw_bytes {
            None => Ok(None),
            Some(raw_bytes) => {
                let (handshake, _) =
                    bincode::serde::decode_from_slice(&raw_bytes, bincode::config::standard())?;
                Ok(Some(handshake))
            }
        }
    }
}

pub struct HandshakeRequestEncoder {
    length_delimited_codec: LengthDelimitedCodec,
}

impl HandshakeRequestEncoder {
    pub fn new() -> Self {
        Self {
            length_delimited_codec: LengthDelimitedCodec::new(),
        }
    }
}

impl Encoder<HandshakeRequest> for HandshakeRequestEncoder {
    type Error = CommonError;
    fn encode(&mut self, item: HandshakeRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let raw_bytes = bincode::serde::encode_to_vec(&item, bincode::config::standard())?;
        self.length_delimited_codec
            .encode(raw_bytes.into(), dst)
            .map_err(Into::into)
    }
}
