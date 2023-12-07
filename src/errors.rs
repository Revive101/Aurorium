use thiserror::Error;

#[derive(Debug, Error)]
pub enum RevisionError {
    #[error("IO Error while reading/writing TcpStream")]
    IO(#[from] std::io::Error),
    #[error("Received invalid MagicHeader sequence")]
    InvalidMagicHeader,
    #[error("Expected SERVICE_ID (8) & MESSAGE_ID (2) but got {0} & {1}")]
    InvalidProtocol(u8, u8),
}
