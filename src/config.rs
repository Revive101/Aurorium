use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, num::NonZeroUsize, path::Path};

use crate::errors::ConfigError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub endpoint: SocketAddr,
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
pub struct DBConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub fetcher: FetcherConfig,
    pub patch: PatchConfig,
    pub database: DBConfig,
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
            database: DBConfig {
                path: "aurorium.db".to_string(),
            },
            debug: None,
        }
    }
}
