use crate::{
    revision::asset_list::AssetList, wizard_patcher::WizardPatcher, xml_parser::parse_file_list,
};
use miette::Diagnostic;
use reqwest::Client;
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;
use tokio::{
    fs::{create_dir_all, try_exists, write},
    io::{AsyncWriteExt, BufWriter},
};
use tracing::{debug, info, instrument};

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
}

pub struct AssetFetcher {
    client: Client,
    concurrent_downloads: NonZeroUsize,
    wizard_patcher: WizardPatcher,
    save_directory: PathBuf,
    assets: AssetList,
}

impl AssetFetcher {
    pub fn new<P>(
        wizard_patcher: WizardPatcher,
        concurrent_downloads: NonZeroUsize,
        save_directory: P,
    ) -> miette::Result<Self>
    where
        P: AsRef<Path>,
    {
        let client = Client::builder()
            .user_agent("KingsIsle Patcher")
            .pool_max_idle_per_host(concurrent_downloads.get())
            .tcp_keepalive(Duration::from_secs(60))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(AssetFetcherError::ClientBuild)?;

        Ok(AssetFetcher {
            client,
            save_directory: save_directory.as_ref().join(&wizard_patcher.revision),
            wizard_patcher,
            concurrent_downloads,
            assets: AssetList::default(),
        })
    }

    #[instrument(skip(self))]
    pub async fn fetch_bin_manifest(&mut self) -> miette::Result<&mut Self> {
        info!("Fetching LatestFileList.bin...");

        let path = self.save_directory.join("LatestFileList.bin");
        let file_exists = try_exists(&path).await.map_err(AssetFetcherError::Io)?;

        if !file_exists {
            let response = self
                .client
                .get(&self.wizard_patcher.list_file_url)
                .send()
                .await
                .map_err(AssetFetcherError::ManifestFetch)?;

            Self::write_to_file_streamed(&path, response).await?;
        } else {
            debug!(path = %path.display(), "BIN manifest already cached, skipping download");
        }

        Ok(self)
    }

    pub async fn fetch_xml_manifest(&mut self) -> miette::Result<&mut Self> {
        info!("Fetching LatestFileList.xml...");

        let path = self.save_directory.join("LatestFileList.xml");
        let file_exists = try_exists(&path).await.map_err(AssetFetcherError::Io)?;
        let list_file_url = self.wizard_patcher.list_file_url.replace(".bin", ".xml");

        if !file_exists {
            let response = self
                .client
                .get(&list_file_url)
                .send()
                .await
                .map_err(AssetFetcherError::ManifestFetch)?;

            Self::write_to_file_streamed(&path, response).await?;
        } else {
            debug!(path = %path.display(), "XML manifest already cached, skipping download");
        }

        let (wads, utils) = parse_file_list(path).unwrap();
        debug!(
            "Parsed {} entries from LatestFileList.xml",
            wads.len() + utils.len()
        );

        self.assets.wads = wads;
        self.assets.utils = utils;

        Ok(self)
    }

    async fn write_to_file<P>(path: P, content: &[u8]) -> miette::Result<()>
    where
        P: AsRef<Path>,
    {
        if let Some(parent) = path.as_ref().parent() {
            create_dir_all(parent)
                .await
                .map_err(AssetFetcherError::CreateDir)?;
        }

        write(path, content).await.map_err(AssetFetcherError::Io)?;

        Ok(())
    }

    async fn write_to_file_streamed<P>(
        path: P,
        mut response: reqwest::Response,
    ) -> miette::Result<()>
    where
        P: AsRef<Path>,
    {
        if let Some(parent) = path.as_ref().parent() {
            create_dir_all(parent)
                .await
                .map_err(AssetFetcherError::CreateDir)?;
        }

        let file = tokio::fs::File::create(path)
            .await
            .map_err(AssetFetcherError::Io)?;
        let mut writer = BufWriter::with_capacity(128 * 1024, file);

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| miette::miette!("Failed to read chunk: {e}"))?
        {
            writer
                .write_all(&chunk)
                .await
                .map_err(AssetFetcherError::Io)?;
        }

        writer.flush().await.map_err(AssetFetcherError::Io)?;

        Ok(())
    }
}
