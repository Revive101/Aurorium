use thiserror::Error;

#[derive(Debug, Error)]
enum KiWadError {
    #[error("Unknown error")]
    Unknown,
    #[error("There was an error while requesting the file")]
    ReqwestError,
    #[error("There was an error while writing to file")]
    Io(#[from] std::io::Error),
}
