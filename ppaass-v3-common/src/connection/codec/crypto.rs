use crate::crypto::{
    decrypt_with_aes, decrypt_with_blowfish, encrypt_with_aes, encrypt_with_blowfish,
};
use crate::error::CommonError;

use ppaass_protocol::Encryption;
use std::sync::Arc;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
pub struct CryptoLengthDelimitedCodec {
    decoder_encryption: Arc<Encryption>,
    encoder_encryption: Arc<Encryption>,
    length_delimited: LengthDelimitedCodec,
}

impl CryptoLengthDelimitedCodec {
    pub fn new(decoder_encryption: Arc<Encryption>, encoder_encryption: Arc<Encryption>) -> Self {
        Self {
            decoder_encryption,
            encoder_encryption,
            length_delimited: LengthDelimitedCodec::new(),
        }
    }
}

impl Decoder for CryptoLengthDelimitedCodec {
    type Item = BytesMut;
    type Error = CommonError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let decrypted_bytes = self.length_delimited.decode(src)?;
        match decrypted_bytes {
            None => Ok(None),
            Some(decrypted_bytes) => match self.decoder_encryption.as_ref() {
                Encryption::Plain => Ok(Some(decrypted_bytes)),
                Encryption::Aes(token) => {
                    let raw_bytes = decrypt_with_aes(&token, &decrypted_bytes)?;
                    Ok(Some(BytesMut::from(raw_bytes)))
                }
                Encryption::Blowfish(token) => {
                    let raw_bytes = decrypt_with_blowfish(&token, &decrypted_bytes)?;
                    Ok(Some(BytesMut::from(raw_bytes)))
                }
            },
        }
    }
}

impl Encoder<BytesMut> for CryptoLengthDelimitedCodec {
    type Error = CommonError;
    fn encode(&mut self, item: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match self.encoder_encryption.as_ref() {
            Encryption::Plain => Ok(self.length_delimited.encode(item.freeze(), dst)?),
            Encryption::Aes(token) => {
                let encrypted_bytes = encrypt_with_aes(token, &item)?;
                Ok(self.length_delimited.encode(encrypted_bytes, dst)?)
            }
            Encryption::Blowfish(token) => {
                let encrypted_bytes = encrypt_with_blowfish(token, &item)?;
                Ok(self.length_delimited.encode(encrypted_bytes, dst)?)
            }
        }
    }
}
