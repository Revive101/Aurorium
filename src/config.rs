use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, num::NonZeroUsize, path::Path};
use thiserror::Error;

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub endpoint: SocketAddr,
    pub max_requests: usize,
    pub reset_interval: u64,
    pub timeout: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FetcherConfig {
    pub concurrent_downloads: NonZeroUsize,
    pub save_directory: String,
    pub fetch_interval: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PatchConfig {
    pub host: String,
    pub port: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugConfig {
    pub level: Option<String>,
    pub file_logging: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub fetcher: FetcherConfig,
    pub patch: PatchConfig,
    pub debug: Option<DebugConfig>,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let path: &Path = Path::new("config.toml");

        if let Ok(content) = std::fs::read_to_string(path) {
            return toml::from_str(&content).map_err(ConfigError::ParseError);
        }

        let default_config = AppConfig::default();
        let toml_string =
            toml::to_string_pretty(&default_config).map_err(ConfigError::SerializeError)?;
        std::fs::write(path, &toml_string).map_err(ConfigError::PathError)?;

        Ok(default_config)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            server: ServerConfig {
                endpoint: SocketAddr::from(([127, 0, 0, 1], 12369)),
                max_requests: 256,
                reset_interval: 60,
                timeout: 10,
            },
            patch: PatchConfig {
                host: "patch.us.wizard101.com".to_string(),
                port: "12500".to_string(),
            },
            fetcher: FetcherConfig {
                concurrent_downloads: unsafe { NonZeroUsize::new_unchecked(2) },
                fetch_interval: 60 * 60 * 8,
                save_directory: "data".to_string(),
            },
            debug: None,
        }
    }
}
