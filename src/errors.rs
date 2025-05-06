#[derive(Debug, thiserror::Error)]
pub enum PatchInfoError {
    #[error("Failed to interact with I/O-Stream")]
    IO(#[from] std::io::Error),

    #[error("Could not resolve address")]
    AddrResolve,

    #[error("Received invalid Header sequence")]
    InvalidHeaderSequence,

    #[error("Expected SERVICE_ID=8 & MESSAGE_ID=2 but got {0} & {1}")]
    InvalidProtocol(u8, u8),

    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Failed to parse revision string")]
    InvalidRevisionFormat,

    #[error("Failed to parse hex string")]
    InvalidHex,
}

#[derive(Debug, thiserror::Error)]
pub enum AssetFetcherError {
    #[error("Failed to parse File")]
    Parse(#[from] std::io::Error),

    #[error("Failed to fetch File")]
    Fetch(#[from] reqwest::Error),

    #[error("Failed to parse XML")]
    XMLParser(#[from] roxmltree::Error),
}
