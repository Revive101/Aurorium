use std::time::Duration;

use crate::{
    config::{AppConfig, FetcherConfig, PatchConfig},
    fetcher::asset_fetcher::AssetFetcher,
    wizard_patcher::WizardPatcher,
};
use miette::Result;
use tokio::time::sleep;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, Layer, fmt::time::ChronoLocal, layer::SubscriberExt, util::SubscriberInitExt,
};

mod config;
mod fetcher;
mod revision;

pub mod utils;
pub mod wizard_patcher;
pub mod xml_parser;

pub struct AppState {
    pub config: AppConfig,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = AppConfig::load()?;

    // Initialize logging
    let _ = init_logging(&config);

    let state = AppState::new(config);

    state.revision_checker().await?;

    Ok(())
}

#[must_use = "The returned logging guard must be stored, so the background thread stays alive!"]
fn init_logging(config: &AppConfig) -> Option<WorkerGuard> {
    let log_level = config
        .debug
        .as_ref()
        .and_then(|debug| debug.level.clone())
        .unwrap_or_else(|| "info".to_string());

    let file_logging = config
        .debug
        .as_ref()
        .and_then(|debug| debug.file_logging.clone())
        .unwrap_or(false);

    let timer = ChronoLocal::new("%d.%m.%Y %H:%M:%S".to_string());

    // Console logging

    let crate_name = env!("CARGO_PKG_NAME").replace('-', "_");
    let console_directive = format!("error,{}={}", crate_name, log_level);

    let console_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(console_directive));

    let console_layer = tracing_subscriber::fmt::layer()
        .with_timer(timer.clone())
        .with_filter(console_filter);

    let mut guard_to_return = None;

    // File logging
    let file_layer = if file_logging {
        let now = chrono::Local::now();
        let file_name = now.format("aurorium_%d-%m-%Y_%H-%M-%S.log").to_string();

        let file_appender = tracing_appender::rolling::never("logs", file_name);
        let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);

        guard_to_return = Some(guard);

        Some(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking_writer)
                .with_timer(timer)
                .with_ansi(false)
                .with_filter(LevelFilter::TRACE),
        )
    } else {
        None
    };

    // Initialize globally, ONCE.
    // If file_layer is None, tracing only uses the console.
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    guard_to_return
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    async fn revision_checker(&self) -> miette::Result<()> {
        let PatchConfig { host, port } = &self.config.patch;
        let FetcherConfig {
            fetch_interval,
            concurrent_downloads,
            save_directory,
            ..
        } = &self.config.fetcher;

        loop {
            let patcher = WizardPatcher::check_revision(&host, &port).await?;

            let mut asset_fetcher =
                AssetFetcher::new(patcher, *concurrent_downloads, save_directory)?;

            asset_fetcher.fetch_bin_manifest().await?;
            asset_fetcher.fetch_xml_manifest().await?;
            asset_fetcher.fetch_assets().await?;

            sleep(Duration::from_secs(*fetch_interval)).await;
        }
    }
}
