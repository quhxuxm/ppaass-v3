use crate::error::CommonError;
use ppaass_protocol::{Encryption, TunnelControlRequest};
use std::sync::Arc;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Decoder;
pub struct TunnelControlRequestDecoder {
    decoder_encryption: Arc<Encryption>,
}

impl TunnelControlRequestDecoder {
    pub fn new(decoder_encryption: Arc<Encryption>) -> Self {
        Self { decoder_encryption }
    }
}

impl Decoder for TunnelControlRequestDecoder {
    type Item = TunnelControlRequest;
    type Error = CommonError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        todo!()
    }
}
