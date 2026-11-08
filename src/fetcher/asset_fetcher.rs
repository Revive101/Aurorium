use crate::{
    errors::AssetFetcherError, fetcher::Fetcher, revision::Asset, wizard_patcher::WizardPatcher,
};
use futures_util::{StreamExt, stream};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::{
        LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::fs::try_exists;
use tracing::{debug, info, instrument, trace, warn};

static MAIN_PROGRESS_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template(
        "{spinner:.yellow} [{pos}/{len}] [{wide_bar:.cyan/blue}] ({eta_precise}/{elapsed_precise})",
    )
    .expect("Failed to create progress bar style")
});

static FILE_PROGRESS_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template("{msg:.cyan} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes}")
        .expect("Failed to create progress bar style")
});

pub struct AssetFetcher<'a> {
    client: Client,
    concurrent_downloads: &'a NonZeroUsize,
    wizard_patcher: WizardPatcher,
    save_directory: PathBuf,
    assets: Vec<Asset>,
}

impl<'a> AssetFetcher<'a> {
    pub fn new<P>(
        wizard_patcher: WizardPatcher,
        concurrent_downloads: &'a NonZeroUsize,
        save_directory: P,
        assets: Vec<Asset>,
    ) -> miette::Result<Self>
    where
        P: AsRef<Path>,
    {
        let client = Client::builder()
            .user_agent("KingsIsle Patcher")
            .pool_max_idle_per_host(concurrent_downloads.get())
            .tcp_keepalive(Duration::from_mins(1))
            .read_timeout(Duration::from_mins(2))
            .build()
            .map_err(AssetFetcherError::ClientBuild)?;

        Ok(AssetFetcher {
            client,
            save_directory: save_directory.as_ref().join(&wizard_patcher.revision.name),
            wizard_patcher,
            concurrent_downloads,
            assets,
        })
    }

    #[instrument(skip(self))]
    pub async fn fetch_assets(&self) -> miette::Result<usize> {
        if self.assets.is_empty() {
            info!(
                "No assets to fetch for revision {}",
                self.wizard_patcher.revision
            );
            return Ok(0);
        }

        debug!(
            "Starting download of {} assets with {} concurrent downloads",
            self.assets.len(),
            self.concurrent_downloads
        );

        let multi_progress = MultiProgress::new();
        let main_progress = multi_progress.add(ProgressBar::new(self.assets.len() as u64));
        main_progress.set_style(MAIN_PROGRESS_STYLE.clone());
        main_progress.enable_steady_tick(Duration::from_millis(200));

        let failed_count = AtomicUsize::new(0);
        let client = &self.client;
        let url_prefix = &self.wizard_patcher.url_prefix;
        let save_dir = &self.save_directory;
        let multi_progress_ref = &multi_progress;
        let main_progress_ref = &main_progress;
        let failed_count_ref = &failed_count;

        let downloads = self.assets.iter().map(|file| async move {
            let url = format!("{url_prefix}/{}", file.file_name);
            let save_path = save_dir.join(&file.file_name);

            trace!(url = %url, file = %file.file_name, "starting download");

            if try_exists(&save_path).await.unwrap_or(false) {
                trace!(file = %file.file_name, "already downloaded, skipping");
                main_progress_ref.inc(1);
                return;
            }

            // Download the file and write it to disk
            match client.get(&url).send().await {
                Ok(res) => {
                    if !res.status().is_success() {
                        warn!(response=res.status().as_u16(), file = %file.file_name, "failed to download asset");
                        failed_count_ref.fetch_add(1, Ordering::Relaxed);
                        main_progress_ref.inc(1);
                        return;
                    }

                    let file_progress = multi_progress_ref.add(ProgressBar::new_spinner());
                    let short_filename =
                        file.file_name.rsplit('/').next().unwrap_or(&file.file_name);
                    file_progress.set_style(FILE_PROGRESS_STYLE.clone());
                    file_progress.set_message(short_filename.to_string());
                    file_progress.set_length(res.content_length().unwrap_or(file.size.into()));

                    match Self::write_to_file_streamed(&save_path, res, Some(&file_progress)).await
                    {
                        Ok(()) => {
                            file_progress.finish_with_message("Done");
                        }
                        Err(e) => {
                            failed_count_ref.fetch_add(1, Ordering::Relaxed);
                            warn!(error = %e, file = %file.file_name, "failed to write file to disk");
                        }
                    }

                    multi_progress_ref.remove(&file_progress);
                }
                Err(e) => {
                    // TODO: Handle retries (or log failures in a separate list)
                    failed_count_ref.fetch_add(1, Ordering::Relaxed);
                    warn!(error = %e, file = %file.file_name, "failed to download file");
                }
            }

            main_progress_ref.inc(1);
        });

        stream::iter(downloads)
            .buffer_unordered(self.concurrent_downloads.get())
            .collect::<Vec<()>>()
            .await;

        let _ = multi_progress.clear();
        let failures = failed_count.load(Ordering::Relaxed);
        info!("All downloads completed ({failures} failed)");

        Ok(failures)
    }
}

impl Fetcher for AssetFetcher<'_> {}
