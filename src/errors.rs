use thiserror::Error;

#[derive(Debug, Error)]
pub enum RevisionCheckerError {
    #[error("IO Error while reading/writing TcpStream")]
    IO(#[from] std::io::Error),
    #[error("Received invalid MagicHeader sequence")]
    InvalidMagicHeader,
    #[error(
        "Expected 'PATCH_SERVICE_ID' (8) PROTOCOL & MSG_LATEST_FILE_LIST_V2 (2) but got {0} & {1}"
    )]
    InvalidProtocol(u8, u8),
}
