use ppaass_protocol::ProtocolError;
use std::net::SocketAddr;
use thiserror::Error;
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
    #[error("Can not find rsa crypto with key: {0}")]
    RsaCryptoNotFound(String),
    #[error("Connection exhausted: {0}")]
    ConnectionExhausted(SocketAddr),
    #[error(transparent)]
    Bincode(#[from] bincode::Error),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error("Other comment error happen: {0}")]
    Other(String),
}
