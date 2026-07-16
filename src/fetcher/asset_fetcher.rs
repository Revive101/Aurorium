use crate::{
    revision::asset_list::AssetList, wizard_patcher::WizardPatcher, xml_parser::parse_file_list,
};
use futures_util::{StreamExt, stream};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use miette::Diagnostic;
use reqwest::Client;
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::LazyLock,
    time::Duration,
};
use thiserror::Error;
use tokio::{
    fs::{create_dir_all, try_exists},
    io::{AsyncWriteExt, BufWriter},
};
use tracing::{debug, info, instrument, trace, warn};

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

static MAIN_PROGRESS_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template(
        "{spinner:.yellow} [{pos}/{len}] [{wide_bar:.cyan/blue}] ({eta_precise}/{elapsed_precise})",
    )
    .unwrap()
});

static FILE_PROGRESS_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template("{msg:.cyan} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes}")
        .unwrap()
});

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

            Self::write_to_file_streamed(&path, response, None).await?;
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

            Self::write_to_file_streamed(&path, response, None).await?;
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

    #[instrument(skip(self))]
    pub async fn fetch_assets(&self) -> miette::Result<()> {
        if self.assets.is_empty() {
            return Err(miette::miette!("No assets to fetch"));
        }

        let total_files = self.assets.wads.len() + self.assets.utils.len();

        debug!(
            "Starting download of {} assets with {} concurrent downloads",
            total_files, self.concurrent_downloads
        );

        let multi_progress = MultiProgress::new();
        let main_progress = multi_progress.add(ProgressBar::new(total_files as u64));
        main_progress.set_style(MAIN_PROGRESS_STYLE.clone());
        main_progress.enable_steady_tick(Duration::from_millis(200));

        let file_list = self.assets.wads.iter().chain(self.assets.utils.iter());
        let downloads = file_list.map(|file| {
            let client = self.client.clone();
            let url_prefix = self.wizard_patcher.url_prefix.clone();
            let save_dir = self.save_directory.clone();

            let multi_progress = multi_progress.clone();
            let main_progress = main_progress.clone();

            async move {
                let url = format!("{url_prefix}/{}", file.file_name);
                let save_path = save_dir.join(&file.file_name);

                if save_path.exists() {
                    trace!(file = %file.file_name, "already downloaded, skipping");
                    main_progress.inc(1);
                    return;
                }

                // Download the file and write it to disk
                let file_progress = multi_progress.add(ProgressBar::new_spinner());
                match client.get(&url).send().await {
                    Ok(res) => {
                        if !res.status().is_success() {
                            debug!(response=res.status().as_u16(), file = %file.file_name, "failed to download asset");
                            main_progress.inc(1);
                            return;
                        }

                        let short_filename = file.file_name.rsplit('/').next().unwrap_or(&file.file_name);
                        file_progress.set_style(FILE_PROGRESS_STYLE.clone());
                        file_progress.set_message(format!("{}", short_filename));
                        file_progress.set_length(res.content_length().unwrap_or(file.size as u64));

                        match Self::write_to_file_streamed(&save_path, res, Some(&file_progress)).await {
                            Ok(_) => {
                                file_progress.finish_with_message("Done");
                            }
                            Err(e) => {
                                warn!(error = %e, file = %file.file_name, "failed to write file to disk");
                            }
                        }

                    },
                    Err(e) => {
                        // TODO: Handle retries (or log failures in a separate list)
                        warn!(error = %e, file = %file.file_name, "failed to download file");
                    }
                }

                main_progress.inc(1);
                multi_progress.remove(&file_progress);
            }
        });

        stream::iter(downloads)
            .buffer_unordered(self.concurrent_downloads.get())
            .collect::<Vec<()>>()
            .await;

        todo!()
    }

    /// Streams an HTTP response to disk, optionally driving a progress bar.
    ///
    /// # Panics
    /// This function will panic if any of the following conditions are met:
    /// - The file path is invalid.
    /// - The response body cannot be read.
    /// - The file cannot be created or written to.
    /// - The file cannot be renamed to its final name after writing.
    async fn write_to_file_streamed<P>(
        path: P,
        mut response: reqwest::Response,
        progress: Option<&ProgressBar>,
    ) -> miette::Result<()>
    where
        P: AsRef<Path>,
    {
        let final_path = path.as_ref();
        let part_path = Self::part_path(final_path);

        // Check if parent dir exists, else create it
        if let Some(parent) = final_path.parent() {
            create_dir_all(parent)
                .await
                .map_err(AssetFetcherError::CreateDir)?;
        }

        let file = tokio::fs::File::create(&part_path)
            .await
            .map_err(AssetFetcherError::Io)?;
        let mut writer = BufWriter::with_capacity(128 * 1024, file);

        // Stream response to file in chunks (to avoid loading the entire file into memory)
        let result: miette::Result<()> = async {
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|e| miette::miette!("Failed to read chunk: {e}"))?
            {
                writer
                    .write_all(&chunk)
                    .await
                    .map_err(AssetFetcherError::Io)?;

                if let Some(pb) = progress {
                    pb.inc(chunk.len() as u64);
                }
            }

            writer.flush().await.map_err(AssetFetcherError::Io)?;
            Ok(())
        }
        .await;

        // If there was an error during the download, remove the partial file and return the error
        if let Err(e) = result {
            let _ = tokio::fs::remove_file(&part_path).await;
            return Err(e);
        }

        // Rename the .part file to the final filename
        tokio::fs::rename(&part_path, final_path)
            .await
            .map_err(AssetFetcherError::Rename)?;

        Ok(())
    }

    fn part_path(path: &Path) -> PathBuf {
        let mut part_os = path.as_os_str().to_owned();
        part_os.push(".part");
        PathBuf::from(part_os)
    }
}
