use ppaass_common::error::CommonError;
use ppaass_protocol::ProtocolError;
use std::net::SocketAddr;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum ProxyError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Common(#[from] CommonError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error("Other error: {0}")]
    Other(String),
}
