use crate::connection::codec::CryptoLengthDelimitedCodec;
use crate::error::CommonError;
use ppaass_protocol::{Encryption, TunnelControlRequest, TunnelControlResponse};
use std::sync::Arc;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};
pub struct TunnelControlResponseRequestCodec {
    crypto_length_delimited_codec: CryptoLengthDelimitedCodec,
}

impl TunnelControlResponseRequestCodec {
    pub fn new(decoder_encryption: Arc<Encryption>, encoder_encryption: Arc<Encryption>) -> Self {
        Self {
            crypto_length_delimited_codec: CryptoLengthDelimitedCodec::new(
                decoder_encryption,
                encoder_encryption,
            ),
        }
    }
}

impl Decoder for TunnelControlResponseRequestCodec {
    type Item = TunnelControlResponse;
    type Error = CommonError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let raw_bytes = self.crypto_length_delimited_codec.decode(src)?;
        match raw_bytes {
            None => Ok(None),
            Some(raw_bytes) => {
                let tunnel_ctl_response: TunnelControlResponse = bincode::deserialize(&raw_bytes)?;
                Ok(Some(tunnel_ctl_response))
            }
        }
    }
}

impl Encoder<TunnelControlRequest> for TunnelControlResponseRequestCodec {
    type Error = CommonError;
    fn encode(
        &mut self,
        item: TunnelControlRequest,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        let raw_bytes = bincode::serialize(&item)?;
        self.crypto_length_delimited_codec
            .encode(raw_bytes.into(), dst)
    }
}
