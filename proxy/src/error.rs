use ppaass_common::error::CommonError;

use std::net::SocketAddr;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum ProxyError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Common(#[from] CommonError),
    #[error("Other error: {0}")]
    Other(String),
}
