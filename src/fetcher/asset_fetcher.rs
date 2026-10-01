use crate::{
    errors::AssetFetcherError, fetcher::asset_download, revision::Asset,
    wizard_patcher::WizardPatcher,
};
use futures_util::{StreamExt, stream};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::LazyLock,
    time::Duration,
};
use tracing::{debug, info, instrument, warn};

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
            .connect_timeout(Duration::from_secs(30))
            // Large WADs may take minutes; only a stalled read should time out.
            .read_timeout(Duration::from_secs(60))
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
    pub async fn fetch_assets(&self) -> miette::Result<()> {
        if self.assets.is_empty() {
            info!(
                "No assets to fetch for revision {}",
                self.wizard_patcher.revision
            );
            return Ok(());
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

        // Fetch headers first: the client needs them even for already-cached WADs.
        let headers = self
            .assets
            .iter()
            .filter(|a| a.compressed_header_size > 0)
            .map(|a| {
                (
                    format!("{}.hdr.gz", a.file_name),
                    u64::from(a.compressed_header_size),
                )
            });
        let assets = self
            .assets
            .iter()
            .map(|a| (a.file_name.clone(), u64::from(a.size)));
        let files: Vec<_> = headers.chain(assets).collect();
        main_progress.set_length(files.len() as u64);
        let downloads = files.into_iter().map(|(name, size)| {
            let client = &self.client;
            let url = format!("{}/{}", self.wizard_patcher.url_prefix, name);
            let path = self.save_directory.join(&name);
            let multi_progress = &multi_progress;
            let main_progress = &main_progress;
            async move {
                let progress = multi_progress.add(ProgressBar::new(size));
                progress.set_style(FILE_PROGRESS_STYLE.clone());
                progress.set_message(name.clone());
                let result = asset_download::download(client, &url, &path, size, &progress).await;
                multi_progress.remove(&progress);
                main_progress.inc(1);
                if let Err(error) = &result {
                    warn!(error = ?error, file = %name, "Asset incomplete; partial transfer retained for retry");
                }
                result
            }
        });

        let results = stream::iter(downloads)
            .buffer_unordered(self.concurrent_downloads.get())
            .collect::<Vec<_>>()
            .await;

        multi_progress.clear().unwrap();
        let failed = results.iter().filter(|result| result.is_err()).count();
        if failed > 0 {
            miette::bail!("{failed} asset downloads failed; they will be retried on the next pass");
        }
        info!("All assets and WAD headers downloaded");

        Ok(())
    }
}
