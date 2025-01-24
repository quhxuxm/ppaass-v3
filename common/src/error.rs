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
}
