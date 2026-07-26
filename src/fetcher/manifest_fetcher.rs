use crate::{
    errors::ManifestFetcherError, fetcher::fetcher::Fetcher, revision::Asset,
    wizard_patcher::WizardPatcher, xml_parser::parse_file_list,
};
use reqwest::Client;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::fs::try_exists;
use tracing::{debug, info};

/// This struct is responsible for fetching the `LatestFileList.bin` and `LatestFileList.xml` from their servers.
pub struct ManifestFetcher {
    client: Client,
    wizard_patcher: WizardPatcher,
    save_directory: PathBuf,
}

impl ManifestFetcher {
    pub fn new<P>(wizard_patcher: WizardPatcher, save_directory: P) -> miette::Result<Self>
    where
        P: AsRef<Path>,
    {
        let client = Client::builder()
            .user_agent("KingsIsle Patcher")
            .tcp_keepalive(Duration::from_mins(1))
            .timeout(Duration::from_mins(2))
            .build()
            .map_err(ManifestFetcherError::ClientBuild)?;

        Ok(Self {
            client,
            wizard_patcher: wizard_patcher.clone(),
            save_directory: save_directory.as_ref().join(wizard_patcher.revision.name),
        })
    }

    pub async fn fetch_bin_manifest(&self) -> miette::Result<()> {
        let path = self.save_directory.join("LatestFileList.bin");
        let file_exists = try_exists(&path).await.map_err(ManifestFetcherError::Io)?;

        if !file_exists {
            info!("Fetching LatestFileList.bin...");
            let response = Self::fetch(&self.client, &self.wizard_patcher.list_file_url).await?;
            Self::write_to_file_streamed(&path, response, None).await?;
            return Ok(());
        }

        info!(path = %path.display(), "BIN manifest already cached, skipping download");
        Ok(())
    }

    pub async fn fetch_xml_manifest(&self) -> miette::Result<Vec<Asset>> {
        let path = self.save_directory.join("LatestFileList.xml");
        let file_exists = try_exists(&path).await.map_err(ManifestFetcherError::Io)?;
        let list_file_url = self.wizard_patcher.list_file_url.replace(".bin", ".xml");

        if !file_exists {
            info!("Fetching LatestFileList.xml...");
            let response = Self::fetch(&self.client, &list_file_url).await?;
            Self::write_to_file_streamed(&path, response, None).await?;
        }

        info!(path = %path.display(), "XML manifest already cached, skipping download");

        let assets = parse_file_list(path).unwrap_or(vec![]);
        debug!("Parsed {} entries from LatestFileList.xml", assets.len());

        if assets.is_empty() {
            return Err(ManifestFetcherError::EmptyAssetList.into());
        }

        Ok(assets)
    }
}

impl Fetcher for ManifestFetcher {}
