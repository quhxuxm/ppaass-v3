use ppaass_protocol::ProtocolError;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::time::error::Elapsed;
use tracing::metadata::ParseLevelError;
#[derive(Error, Debug)]

pub enum CommonError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Aes crypto error: {_0}")]
    Aes(String),
    #[error("Rsa crypto error: {_0}")]
    Rsa(String),
    #[error(transparent)]
    ParseLogLevel(#[from] ParseLevelError),
    #[error("Can not find agent_user crypto with key: {0}")]
    RsaCryptoNotFound(String),
    #[error("User expired: {0}")]
    UserExpired(String),
    #[error("Connection exhausted: {0}")]
    ConnectionExhausted(SocketAddr),
    #[error(transparent)]
    BincodeEncode(#[from] bincode::error::EncodeError),
    #[error(transparent)]
    BincodeDecode(#[from] bincode::error::DecodeError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error(transparent)]
    Timeout(#[from] Elapsed),
    #[error("Other comment error happen: {0}")]
    Other(String),
}

impl From<CommonError> for std::io::Error {
    fn from(value: CommonError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, value)
    }
}
