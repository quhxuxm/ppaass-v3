use thiserror::Error;
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Failed to parse unified address to domain: {0:?}")]
    ParseUnifiedAddressToDomainAddress(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
