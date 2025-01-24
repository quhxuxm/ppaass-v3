pub mod crypto;
pub mod error;
use rand::random;
use uuid::Uuid;

/// Generate a random UUID
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string().replace("-", "").to_uppercase()
}

/// Generate a random 32 bytes vector
pub fn random_32_bytes() -> Vec<u8> {
    let random_32_bytes = random::<[u8; 32]>();
    random_32_bytes.to_vec()
}
