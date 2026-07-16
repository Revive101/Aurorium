use miette::Diagnostic;
use thiserror::Error;

// asset_fetcher.rs
#[derive(Diagnostic, Error, Debug)]
pub enum AssetFetcherError {
    #[error("Failed to create HTTP client")]
    #[diagnostic(
        code(asset_fetcher::client_build),
        help(
            "There was an error while creating the HTTP client. Please restart Aurorium or try again later."
        )
    )]
    ClientBuild(#[source] reqwest::Error),

    #[error("Failed to create directories")]
    #[diagnostic(
        code(asset_fetcher::create_dir),
        help(
            "There was an error while creating directories for storing assets. Please check your file system permissions and try again."
        )
    )]
    CreateDir(#[source] std::io::Error),

    #[error("File system I/O error")]
    #[diagnostic(code(asset_fetcher::io))]
    Io(#[source] std::io::Error),

    #[error("Failed to fetch LatestFileList")]
    #[diagnostic(code(asset_fetcher::manifest_fetch))]
    ManifestFetch(#[source] reqwest::Error),

    #[error("failed to finalize downloaded file")]
    #[diagnostic(code(asset_fetcher::rename))]
    Rename(#[source] std::io::Error),
}

// xml_parser.rs
#[derive(Error, Diagnostic, Debug)]
pub enum XmlParseError {
    #[error("Failed to open XML file")]
    #[diagnostic(
        code(xml_parser::file_open),
        help(
            "There was an error while opening the XML file. Please check if the file exists and you have the necessary permissions."
        )
    )]
    FileOpen(#[source] std::io::Error),

    #[error("Failed to read XML file")]
    #[diagnostic(
        code(xml_parser::read),
        help(
            "There was an error while reading the XML file. Please check if the file is accessible and not corrupted."
        )
    )]
    Read(#[source] quick_xml::Error),

    #[error("Failed to parse integer from XML content")]
    #[diagnostic(
        code(xml_parser::parse_int),
        help(
            "There was an error while parsing an integer from the XML content. Please check if the XML file is well-formed and contains valid data."
        )
    )]
    Parse(#[from] std::num::ParseIntError),

    #[error("Failed to parse XML content")]
    #[diagnostic(
        code(xml_parser::parse),
        help(
            "There was an error while parsing the XML content. Please check if the XML file is well-formed and contains valid data."
        )
    )]
    Encoding(#[source] quick_xml::encoding::EncodingError),
}

// local_revision.rs
#[derive(Debug, Error, Diagnostic)]
pub enum LocalRevisionError {}

// wizard_patcher.rs
#[derive(Debug, Error, Diagnostic)]
pub enum WizardPatcherError {
    #[error("Failed to connect to the server")]
    #[diagnostic(
        code(wizard_patcher::connection_error),
        help("Check your internet connection and try again.")
    )]
    ConnectionError(#[source] std::io::Error),

    #[error("Failed to read from the server")]
    #[diagnostic(
        code(wizard_patcher::read_error),
        help(
            "There was an unknown error while reading data from the server. Please restart Aurorium or try again later."
        )
    )]
    ReadError(#[source] std::io::Error),

    #[error("Failed to write to the server")]
    #[diagnostic(
        code(wizard_patcher::write_error),
        help(
            "There was an unknown error while writing data to the server. Please restart Aurorium or try again later."
        )
    )]
    WriteError(#[source] std::io::Error),

    #[error("Failed to shutdown the connection")]
    #[diagnostic(
        code(wizard_patcher::shutdown_error),
        help(
            "There was an unknown error while shutting down the connection. Please restart Aurorium or try again later."
        )
    )]
    ShutdownError(#[source] std::io::Error),

    #[error("Unexpected end of file while reading response")]
    #[diagnostic(
        code(wizard_patcher::unexpected_eof),
        help(
            "Encountered an EOF while filling the response buffer. This may indicate a network issue or a problem with Aurorium. Please try again later."
        )
    )]
    UnexpectedEofError(#[source] std::io::Error),
}
