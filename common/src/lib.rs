pub mod config;
mod connection;
pub mod crypto;
pub mod error;
pub mod server;
use crate::crypto::{generate_aes_encryption_token, generate_blowfish_encryption_token, RsaCrypto};
use crate::error::CommonError;
pub use connection::*;
pub use ppaass_protocol::*;
use rand::random;
use std::borrow::Cow;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::str::FromStr;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::time::ChronoUtc;
use uuid::Uuid;
/// Generate a random UUID
#[inline(always)]
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string().replace("-", "").to_uppercase()
}

/// Randomly generate a raw encryption
#[inline(always)]
pub fn random_generate_encryption() -> Encryption {
    let random_number = random::<u64>();
    if random_number % 2 == 0 {
        Encryption::Aes(generate_aes_encryption_token())
    } else {
        Encryption::Blowfish(generate_blowfish_encryption_token())
    }
}

#[inline(always)]
pub fn rsa_encrypt_encryption<'a>(
    raw_encryption: &'a Encryption,
    rsa_crypto: &RsaCrypto,
) -> Result<Cow<'a, Encryption>, CommonError> {
    match raw_encryption {
        Encryption::Plain => Ok(Cow::Borrowed(raw_encryption)),
        Encryption::Aes(token) => {
            let encrypted_token = rsa_crypto.encrypt(&token)?;
            Ok(Cow::Owned(Encryption::Aes(encrypted_token)))
        }
        Encryption::Blowfish(token) => {
            let encrypted_token = rsa_crypto.encrypt(&token)?;
            Ok(Cow::Owned(Encryption::Blowfish(encrypted_token)))
        }
    }
}

#[inline(always)]
pub fn rsa_decrypt_encryption<'a>(
    encrypted_encryption: &'a Encryption,
    rsa_crypto: &RsaCrypto,
) -> Result<Cow<'a, Encryption>, CommonError> {
    match encrypted_encryption {
        Encryption::Plain => Ok(Cow::Borrowed(encrypted_encryption)),
        Encryption::Aes(token) => {
            let decrypted_token = rsa_crypto.decrypt(&token)?;
            Ok(Cow::Owned(Encryption::Aes(decrypted_token)))
        }
        Encryption::Blowfish(token) => {
            let decrypted_token = rsa_crypto.decrypt(&token)?;
            Ok(Cow::Owned(Encryption::Blowfish(decrypted_token)))
        }
    }
}

/// Init the logger
pub fn init_logger(
    // The folder to store the log file
    log_folder: &Path,
    // The log name prefix
    log_name_prefix: &str,
    // The max log level
    max_log_level: &str,
) -> Result<WorkerGuard, CommonError> {
    let (trace_file_appender, _trace_appender_guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::daily(log_folder, log_name_prefix),
    );
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(max_log_level)?)
        .with_writer(trace_file_appender)
        .with_line_number(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_timer(ChronoUtc::rfc_3339())
        .with_ansi(false)
        .init();
    Ok(_trace_appender_guard)
}

#[inline(always)]
pub fn parse_to_socket_addresses<I, T>(addresses: I) -> Result<Vec<SocketAddr>, CommonError>
where
    I: Iterator<Item = T>,
    T: AsRef<str>,
{
    let proxy_addresses = addresses
        .into_iter()
        .filter_map(|addr| addr.as_ref().to_socket_addrs().ok())
        .flatten()
        .collect::<Vec<SocketAddr>>();
    Ok(proxy_addresses)
}
