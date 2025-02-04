use crate::error::CommonError;

use crate::crypto::random_n_bytes;
use aes::Aes256;
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use hyper::body::Bytes;
/// Generate the encryption token for AES
#[inline(always)]
pub(crate) fn generate_aes_encryption_token() -> Bytes {
    random_n_bytes::<32>()
}

/// Encrypt the target bytes with AES
#[inline(always)]
pub fn encrypt_with_aes(encryption_token: &[u8], target: &[u8]) -> Result<Bytes, CommonError> {
    let aes_encryptor = Aes256::new(encryption_token.into());
    let result = aes_encryptor.encrypt_padded_vec::<Pkcs7>(target);
    Ok(result.into())
}

/// Decrypt the target bytes with AES
#[inline(always)]
pub fn decrypt_with_aes(encryption_token: &[u8], target: &[u8]) -> Result<Bytes, CommonError> {
    let aes_decrypt = Aes256::new(encryption_token.into());
    let result = aes_decrypt
        .decrypt_padded_vec::<Pkcs7>(target)
        .map_err(|e| CommonError::Aes(format!("Fail to decrypt with aes block: {e:?}")))?;
    Ok(result.into())
}
#[test]
fn test() -> Result<(), CommonError> {
    let encryption_token = generate_aes_encryption_token();
    let target = "hello world! this is my plaintext.".as_bytes().to_vec();
    let encrypt_result = encrypt_with_aes(&encryption_token, &target)?;
    println!(
        "Encrypt result: [{:?}]",
        String::from_utf8_lossy(&encrypt_result)
    );
    let decrypted_result = decrypt_with_aes(&encryption_token, &encrypt_result)?;
    println!(
        "Decrypted result: [{:?}]",
        String::from_utf8_lossy(&decrypted_result)
    );
    Ok(())
}
