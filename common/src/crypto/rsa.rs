use crate::error::CommonError;
use rand::rngs::OsRng;
use rsa::{
    pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding},
    Pkcs1v15Encrypt,
};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::{fmt::Debug, path::Path};
use std::{fs, io::Read};
const DEFAULT_AGENT_PRIVATE_KEY_PATH: &str = "AgentPrivateKey.pem";
const DEFAULT_AGENT_PUBLIC_KEY_PATH: &str = "AgentPublicKey.pem";
const DEFAULT_SERVER_PRIVATE_KEY_PATH: &str = "ServerPrivateKey.pem";
const DEFAULT_SERVER_PUBLIC_KEY_PATH: &str = "ServerPublicKey.pem";
/// The util to do RSA encryption and decryption.
#[derive(Debug)]
pub struct RsaCrypto {
    /// The private used to do decryption
    private_key: RsaPrivateKey,
    /// The public used to do encryption
    public_key: RsaPublicKey,
}
impl RsaCrypto {
    pub fn new<A, B>(mut public_key_read: A, mut private_key_read: B) -> Result<Self, CommonError>
    where
        A: Read + Debug,
        B: Read + Debug,
    {
        let mut public_key_string = String::new();
        public_key_read.read_to_string(&mut public_key_string)?;
        let public_key = RsaPublicKey::from_public_key_pem(&public_key_string)
            .map_err(|e| CommonError::Rsa(format!("Fail to create rsa public key: {e:?}")))?;
        let mut private_key_string = String::new();
        private_key_read.read_to_string(&mut private_key_string)?;
        let private_key = RsaPrivateKey::from_pkcs8_pem(&private_key_string)
            .map_err(|e| CommonError::Rsa(format!("Fail to create rsa private key: {e:?}")))?;
        Ok(Self {
            public_key,
            private_key,
        })
    }

    /// Encrypt the target bytes with RSA public key
    pub fn encrypt(&self, target: &[u8]) -> Result<Vec<u8>, CommonError> {
        let result = self
            .public_key
            .encrypt(&mut OsRng, Pkcs1v15Encrypt, target.as_ref())
            .map_err(|e| CommonError::Rsa(format!("Fail to encrypt with rsa: {e:?}")))?;
        Ok(result)
    }

    /// Decrypt the target bytes with RSA private key
    pub fn decrypt(&self, target: &[u8]) -> Result<Vec<u8>, CommonError> {
        let result = self
            .private_key
            .decrypt(Pkcs1v15Encrypt, target.as_ref())
            .map_err(|e| CommonError::Rsa(format!("Fail to decrypt with rsa: {e:?}")))?;
        Ok(result)
    }
}

/// Generate the key pairs for agent
pub fn generate_agent_key_pairs(base_dir: &str, auth_token: &str) -> Result<(), CommonError> {
    let private_key_path = format!("{base_dir}/{auth_token}/{DEFAULT_AGENT_PRIVATE_KEY_PATH}");
    let private_key_path = Path::new(private_key_path.as_str());
    let public_key_path = format!("{base_dir}/{auth_token}/{DEFAULT_AGENT_PUBLIC_KEY_PATH}");
    let public_key_path = Path::new(public_key_path.as_str());
    generate_rsa_key_pairs(private_key_path, public_key_path)
}

/// Generate the key pairs for server
pub fn generate_server_key_pairs(base_dir: &str, auth_token: &str) -> Result<(), CommonError> {
    let private_key_path = format!("{base_dir}/{auth_token}/{DEFAULT_SERVER_PRIVATE_KEY_PATH}");
    let private_key_path = Path::new(private_key_path.as_str());
    let public_key_path = format!("{base_dir}/{auth_token}/{DEFAULT_SERVER_PUBLIC_KEY_PATH}");
    let public_key_path = Path::new(public_key_path.as_str());
    generate_rsa_key_pairs(private_key_path, public_key_path)
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
