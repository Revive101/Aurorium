use thiserror::Error;

#[derive(Debug, Error)]
pub enum RevisionError {
    #[error("Failed to interact with I/O-Stream")]
    IO(#[from] std::io::Error),
    #[error("Received invalid Header sequence")]
    InvalidHeaderSequence,
    #[error("Expected SERVICE_ID=8 & MESSAGE_ID=2 but got {0} & {1}")]
    InvalidProtocol(u8, u8),
}
