use ppaass_common::error::CommonError;

use thiserror::Error;
#[derive(Error, Debug)]
pub enum AgentError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Bincode(#[from] bincode::Error),
    #[error(transparent)]
    Common(#[from] CommonError),
}
