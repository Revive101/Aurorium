use crate::{
    config::{AppConfig, FetcherConfig, PatchConfig, ServerConfig},
    db::Database,
    fetcher::{asset_fetcher::AssetFetcher, manifest_fetcher::ManifestFetcher},
    revision::Revision,
    routes::{file::file, latest::get_latest_revision, revisions::get_revisions},
    wizard_patcher::WizardPatcher,
    xml_parser::parse_file_list,
};
use axum::{Router, routing::get};
use miette::Result;
use std::{net::SocketAddr, time::Duration};
use tokio::{net::TcpListener, time::sleep};
use tracing::{debug, info, level_filters::LevelFilter, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, Layer, fmt::time::ChronoLocal, layer::SubscriberExt, util::SubscriberInitExt,
};

pub mod db;
pub mod errors;
pub mod utils;
pub mod wizard_patcher;
pub mod xml_parser;

mod config;
mod fetcher;
mod revision;
mod routes;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: Database,
}

impl AppState {
    pub fn new(config: AppConfig, db: Database) -> Self {
        Self { config, db }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = AppConfig::load()?;

    // Initialize logging
    let _logging = init_logging(&config);

    // Initialize database
    let db = Database::init(&config.database.path).await?;

    let state = AppState::new(config.clone(), db.clone());
    let tasks = tokio::join!(revision_checker(config, db), file_server(state));

    tasks.0?;
    tasks.1?;

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
        .and_then(|debug| debug.file_logging)
        .unwrap_or(false);

    let timer = ChronoLocal::new("%d.%m.%Y %H:%M:%S".to_string());

    // Console logging
    let crate_name = env!("CARGO_PKG_NAME").replace('-', "_");
    let console_directive = format!("error,{crate_name}={log_level}");

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

async fn revision_checker(config: AppConfig, db: Database) -> miette::Result<()> {
    let PatchConfig { host, port } = &config.patch;
    let FetcherConfig {
        fetch_interval,
        concurrent_downloads,
        save_directory,
        ..
    } = &config.fetcher;

    loop {
        info!("Checking for a new revision @ {host}:{port}");

        let wizard_patcher = WizardPatcher::check_revision(host, port).await?;
        let manifest_fetcher = ManifestFetcher::new(wizard_patcher.clone(), save_directory)?;
        manifest_fetcher.fetch_bin_manifest().await?;
        let new_assets = manifest_fetcher.fetch_xml_manifest().await?;

        match db
            .insert_new_revision(wizard_patcher.revision.clone(), new_assets)
            .await
        {
            Ok(assets) => {
                info!(
                    "Revision {} has {} updated or new assets. Starting/Continuing download...",
                    &wizard_patcher.revision,
                    assets.len()
                );

                let asset_fetched =
                    AssetFetcher::new(wizard_patcher, concurrent_downloads, save_directory, assets)
                        .unwrap();

                asset_fetched.fetch_assets().await?;
            }
            Err(e) => {
                warn!(error = %e, "Failed to insert new revision into database");
            }
        }

        info!("Done checking. Sleeping...");
        sleep(Duration::from_secs(*fetch_interval)).await;
    }
}

async fn file_server(state: AppState) -> miette::Result<()> {
    let ServerConfig { endpoint, .. } = &state.config.server;

    let app = Router::new()
        .route("/revisions", get(get_revisions))
        .route("/latest", get(get_latest_revision))
        .route("/{revision}/{*file_path}", get(file))
        .with_state(state.clone());

    let listener = TcpListener::bind(&endpoint).await.unwrap();
    info!("File server listening on {}", &endpoint);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();

    Ok(())
}
