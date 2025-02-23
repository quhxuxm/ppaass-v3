use ppaass_common::crypto::{
    EncodePrivateKey, EncodePublicKey, LineEnding, OsRng, RsaPrivateKey, RsaPublicKey,
    DEFAULT_AGENT_PRIVATE_KEY_PATH, DEFAULT_AGENT_PUBLIC_KEY_PATH, DEFAULT_PROXY_PRIVATE_KEY_PATH,
    DEFAULT_PROXY_PUBLIC_KEY_PATH,
};
use ppaass_common::error::CommonError;
use std::fs;
use std::path::Path;
/// Generate the key pairs for agent
pub fn generate_agent_key_pairs(base_dir: &Path, username: &str) -> Result<(), CommonError> {
    let private_key_path = base_dir.join(username).join(DEFAULT_AGENT_PRIVATE_KEY_PATH);
    let public_key_path = base_dir.join(username).join(DEFAULT_AGENT_PUBLIC_KEY_PATH);
    generate_rsa_key_pairs(&private_key_path, &public_key_path)
}

/// Generate the key pairs for proxy
pub fn generate_proxy_key_pairs(base_dir: &Path, username: &str) -> Result<(), CommonError> {
    let private_key_path = base_dir.join(username).join(DEFAULT_PROXY_PRIVATE_KEY_PATH);
    let public_key_path = base_dir.join(username).join(DEFAULT_PROXY_PUBLIC_KEY_PATH);
    generate_rsa_key_pairs(&private_key_path, &public_key_path)
}
fn generate_rsa_key_pairs(
    private_key_path: &Path,
    public_key_path: &Path,
) -> Result<(), CommonError> {
    let private_key = RsaPrivateKey::new(&mut OsRng, 2048).expect("Fail to generate private key");
    let public_key = RsaPublicKey::from(&private_key);
    let private_key_pem = private_key
        .to_pkcs8_pem(LineEnding::CRLF)
        .expect("Fail to generate pem for private key.");
    let public_key_pem = public_key
        .to_public_key_pem(LineEnding::CRLF)
        .expect("Fail to generate pem for public key.");
    match private_key_path.parent() {
        None => {
            println!("Write private key: {:?}", private_key_path.to_str());
            fs::write(private_key_path, private_key_pem.as_bytes())?;
        }
        Some(parent) => {
            if !parent.exists() {
                println!("Create parent directory :{:?}", parent.to_str());
                fs::create_dir_all(parent)?;
            }
            println!("Write private key: {:?}", private_key_path.to_str());
            fs::write(private_key_path, private_key_pem.as_bytes())?;
        }
    };
    match public_key_path.parent() {
        None => {
            println!("Write public key: {:?}", public_key_path.to_str());
            fs::write(public_key_path, public_key_pem.as_bytes())?;
        }
        Some(parent) => {
            if !parent.exists() {
                println!("Create parent directory :{:?}", parent.to_str());
                fs::create_dir_all(parent)?;
            }
            println!("Write public key: {:?}", public_key_path.to_str());
            fs::write(public_key_path, public_key_pem.as_bytes())?;
        }
    };
    Ok(())
}
