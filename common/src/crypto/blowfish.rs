use crate::crypto::random_n_bytes;
use crate::error::CommonError;
use blowfish::Blowfish;
use bytes::Bytes;
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};

type BlowfishCbcEncryptor = cbc::Encryptor<Blowfish>;
type BlowfishCbcDecryptor = cbc::Decryptor<Blowfish>;
#[inline(always)]
pub(crate) fn generate_blowfish_encryption_token() -> Bytes {
    random_n_bytes::<64>()
}

/// Encrypt the target bytes with Blowfish
#[inline(always)]
pub fn encrypt_with_blowfish(encryption_token: &[u8], target: &[u8]) -> Result<Bytes, CommonError> {
    let encryptor =
        BlowfishCbcEncryptor::new_from_slices(&encryption_token[..56], &encryption_token[56..])
            .map_err(|e| {
                CommonError::Other(format!("Fail to generate blowfish encryptor: {e:?}"))
            })?;
    let result = encryptor.encrypt_padded_vec_mut::<Pkcs7>(target);
    Ok(result.into())
}

/// Decrypt the target bytes with Blowfish
#[inline(always)]
pub fn decrypt_with_blowfish(encryption_token: &[u8], target: &[u8]) -> Result<Bytes, CommonError> {
    let decryptor =
        BlowfishCbcDecryptor::new_from_slices(&encryption_token[..56], &encryption_token[56..])
            .map_err(|e| {
                CommonError::Other(format!("Fail to generate blowfish decryptor: {e:?}"))
            })?;
    let result = decryptor
        .decrypt_padded_vec_mut::<Pkcs7>(target)
        .map_err(|e| CommonError::Aes(format!("Fail to decrypt with blowfish block: {e:?}")))?;
    Ok(result.into())
}

#[test]
fn test() -> Result<(), CommonError> {
    let encryption_token = generate_blowfish_encryption_token();
    let target = "hello world! this is my plaintext.".as_bytes().to_vec();
    let encrypt_result = encrypt_with_blowfish(&encryption_token, &target)?;
    println!(
        "Encrypt result: [{:?}]",
        String::from_utf8_lossy(&encrypt_result)
    );
    let decrypted_result = decrypt_with_blowfish(&encryption_token, &encrypt_result)?;
    println!(
        "Decrypted result: [{:?}]",
        String::from_utf8_lossy(&decrypted_result)
    );
    Ok(())
}
