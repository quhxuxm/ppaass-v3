use ppaass_common::error::CommonError;
use ppaass_protocol::ProtocolError;
use std::net::SocketAddr;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum AgentError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Bincode(#[from] bincode::Error),
    #[error(transparent)]
    Common(#[from] CommonError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("Can not find rsa crypto with key: {0}")]
    RsaCryptoNotFound(String),
    #[error("Agent connection exhausted: {0}")]
    AgentConnectionExhausted(SocketAddr),
    #[error("Other error: {0}")]
    Other(String),
}
