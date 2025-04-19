use ppaass_common::error::CommonError;

use thiserror::Error;
#[derive(Error, Debug)]
pub enum AgentError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    BincodeEncode(#[from] bincode::error::EncodeError),
    #[error(transparent)]
    BincodeDecode(#[from] bincode::error::DecodeError),
    #[error(transparent)]
    Common(#[from] CommonError),
}
