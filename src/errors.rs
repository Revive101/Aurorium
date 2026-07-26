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

    #[error("Unexpected response length: {0}")]
    #[diagnostic(
        code(wizard_patcher::unexpected_response_length),
        help(
            "The server response length was unexpectedly {0} while {1} was expected. This may indicate a change in the server's response format. Please check for updates or report this issue."
        )
    )]
    UnexpectedResponseLength(usize, usize),

    #[error("Failed to parse revision: {0}")]
    #[diagnostic(
        code(wizard_patcher::revision_parse_error),
        help(
            "Failed to parse the revision ({0}) from the server response. This may indicate a change in the server's response format. Please check for updates or report this issue."
        )
    )]
    RevisionParseError(String),

    #[error("Revision number must be non-negative, got {0}")]
    #[diagnostic(
        code(wizard_patcher::invalid_revision_number),
        help("Pass a revision number >= 0.")
    )]
    InvalidRevisionNumber(i64),
}

// manifest_fetcher.rs
#[derive(Debug, Error, Diagnostic)]
pub enum ManifestFetcherError {
    #[error("File system I/O error")]
    #[diagnostic(code(asset_fetcher::io))]
    Io(#[source] std::io::Error),

    #[error("Failed to create HTTP client")]
    #[diagnostic(
        code(asset_fetcher::client_build),
        help(
            "There was an error while creating the HTTP client. Please restart Aurorium or try again later."
        )
    )]
    ClientBuild(#[source] reqwest::Error),

    #[error("Parsed asset list is empty")]
    #[diagnostic(
        code(asset_fetcher::empty_asset_list),
        help(
            "The parsed asset list is empty. This may indicate that the XML file is invalid or empty. Please check the XML file and try again."
        )
    )]
    EmptyAssetList,
}

// fetcher.rs
#[derive(Debug, Error, Diagnostic)]
pub enum FetcherTraitError {
    #[error("Failed to fetch {1}")]
    #[diagnostic(code(asset_fetcher::manifest_fetch))]
    Fetch(#[source] reqwest::Error, String),

    #[error("File system I/O error")]
    #[diagnostic(code(asset_fetcher::io))]
    Io(#[source] std::io::Error),

    #[error("Failed to create directories")]
    #[diagnostic(
        code(asset_fetcher::create_dir),
        help(
            "There was an error while creating directories. Please check your file system permissions and try again."
        )
    )]
    CreateDir(#[source] std::io::Error),

    #[error("failed to finalize downloaded file")]
    #[diagnostic(code(asset_fetcher::rename))]
    Rename(#[source] std::io::Error),
}

// config.rs
#[derive(Debug, Error, Diagnostic)]
pub enum ConfigError {
    #[error("Could not find config.toml")]
    #[diagnostic(
        code(config::not_found),
        help("Create a config.toml file in the current directory")
    )]
    NotFound(#[source] std::io::Error),

    #[error("Failed to parse config.toml")]
    #[diagnostic(
        code(config::parse_error),
        help(
            "Check your config.toml for any missing or invalid fields, check the documentation for reference!"
        )
    )]
    ParseError(#[source] toml::de::Error),

    #[error("Failed to serialize default config.toml")]
    #[diagnostic(
        code(config::serialize_error),
        help("This is usually caused by an invalid default configuration value")
    )]
    SerializeError(#[source] toml::ser::Error),

    #[error("Failed to write default config.toml")]
    #[diagnostic(
        code(config::write_error),
        help("Check your permissions and try again")
    )]
    PathError(#[source] std::io::Error),
}

#[derive(Debug, Error, Diagnostic)]
pub enum DbError {
    #[error("Database initialization error: {0}")]
    #[diagnostic(
        code(db::init_db_error),
        help("Ensure the database path is correct and accessible.")
    )]
    Init(rusqlite::Error),

    #[error("Database execution error: {0}")]
    #[diagnostic(
        code(db::execute_error),
        help("Check the SQL statement for correctness. (This should NOT happen!)")
    )]
    Execute(#[from] rusqlite::Error),

    #[error("Database transaction error: {0}")]
    #[diagnostic(
        code(db::transaction_error),
        help("Ensure that the transaction is valid and that the database is not locked.")
    )]
    Transaction(rusqlite::Error),

    #[error("Database migration error: {0}")]
    #[diagnostic(
        code(db::migration_error),
        help("Check the migration scripts for correctness. (This should NOT happen!)")
    )]
    Migration(#[from] rusqlite_migration::Error),

    #[error("Async SQLite error: {0}")]
    #[diagnostic(
        code(db::async_sqlite_error),
        help(
            "Ensure that the async SQLite client is properly configured and that the database is accessible."
        )
    )]
    AsyncSqlite(#[from] async_sqlite::Error),

    #[error("Revision number must be non-negative, got {0}")]
    #[diagnostic(
        code(db::invalid_revision_number),
        help("Pass a revision number >= 0.")
    )]
    InvalidRevisionNumber(i64),
}

// routes/*.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum RouteError {
    #[error("File not found: {0}")]
    #[diagnostic(
        code(route::file_not_found),
        help("Ensure the requested file exists on the server.")
    )]
    NotFound(String),

    #[error("Database error: {0}")]
    #[diagnostic(
        code(route::database_error),
        help("Check the database connection and query for correctness.")
    )]
    Database(#[from] DbError),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Invalid working directory")]
    #[diagnostic(
        code(route::invalid_working_dir),
        help("The server's working directory is invalid. Please check the server configuration.")
    )]
    InvalidWorkingDir(#[from] std::io::Error),
}
