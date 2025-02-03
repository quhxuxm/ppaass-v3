use crate::error::CommonError;

use crate::crypto::random_n_bytes;
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
pub(crate) fn generate_blowfish_encryption_token() -> Vec<u8> {
    random_n_bytes::<56>()
}

/// Encrypt the target bytes with Blowfish
pub fn encrypt_with_blowfish(
    encryption_token: &[u8],
    target: &[u8],
) -> Result<Vec<u8>, CommonError> {
    let blowfish_encryptor: blowfish::Blowfish = blowfish::Blowfish::new(encryption_token.into());
    let result = blowfish_encryptor.encrypt_padded_vec::<Pkcs7>(target);
    Ok(result)
}

/// Decrypt the target bytes with Blowfish
pub fn decrypt_with_blowfish(
    encryption_token: &[u8],
    target: &[u8],
) -> Result<Vec<u8>, CommonError> {
    let blowfish_encryptor: blowfish::Blowfish = blowfish::Blowfish::new(encryption_token.into());
    let result = blowfish_encryptor
        .decrypt_padded_vec::<Pkcs7>(target)
        .map_err(|e| CommonError::Aes(format!("Fail to decrypt with blowfish block: {e:?}")))?;
    Ok(result)
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
