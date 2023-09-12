use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum KiWadError {
    #[error("Unknown error")]
    Unknown,
    #[error("Could not request file")]
    ReqwestError,
    #[error("Could not write to file")]
    Io(#[from] std::io::Error),
}
