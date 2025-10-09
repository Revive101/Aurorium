use anyhow::Context;
use axum::{Router, error_handling::HandleErrorLayer, routing::get};
use clap::Parser;
use fetcher::{client::AssetFetcher, compare::compare_revisions};
use models::revision::LocalRevision;
use patch_info::PatchInfo;
use routes::{file, handle_error, revisions};
use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    num::NonZeroUsize,
    path::PathBuf,
    sync::LazyLock,
    time::Duration,
};
use tokio::{net::TcpListener, sync::RwLock, time::sleep};
use tower::{ServiceBuilder, buffer::BufferLayer, limit::RateLimitLayer, timeout::TimeoutLayer};

pub mod fetcher;
pub mod models;
pub mod patch_info;
mod routes;
pub mod utils;
pub mod xml_parser;

const DEFAULT: &str = "patch.us.wizard101.com";
const DEFAULT_PORT: &str = "12500";

pub static REVISIONS: LazyLock<RwLock<HashSet<LocalRevision>>> = LazyLock::new(|| RwLock::new(HashSet::new()));
pub static ARGS: LazyLock<Args> = LazyLock::new(|| Args::parse());

#[derive(Clone, Parser)]
#[clap(author, version, about)]
pub struct Args {
    #[arg(short, long, env = "ENDPOINT", default_value_t = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 12369))]
    endpoint: SocketAddrV4,

    #[arg(short, long, env = "CONCURRENT_DOWNLOADS", default_value_t = unsafe { NonZeroUsize::new_unchecked(2) })]
    concurrent_downloads: NonZeroUsize,

    #[arg(short, long, env = "SAVE_DIRECTORY", default_value = "data")]
    save_directory: PathBuf,

    #[arg(long, env = "HOST", default_value = DEFAULT)]
    patch_host: String,

    #[arg(long, env = "PORT", default_value = DEFAULT_PORT)]
    patch_port: String,

    #[arg(short, long, default_value_t = 60 * 60 * 8)]
    fetch_interval: u64,

    #[arg(short, long, default_value_t = 256)]
    max_requests: u64,

    #[arg(short, long, default_value_t = 60)]
    reset_interval: u64,

    #[arg(short, long, default_value_t = 10)]
    timeout: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize all revisions on disk
    LocalRevision::init_all(&ARGS.save_directory).await?;

    let server_handle = tokio::spawn(run_file_server());
    let checker_handle = tokio::spawn(revision_checker());

    // Wait for both tasks and handle errors
    let (server_result, checker_result) = tokio::join!(server_handle, checker_handle);

    // Check for task panics or errors
    server_result.context("File server task panicked")??;
    checker_result.context("Revision checker task panicked")??;

    Ok(())
}

async fn run_file_server() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/{revision}/{*file_path}", get(file))
        .route("/revisions", get(revisions))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_error))
                .layer(BufferLayer::new(1024))
                .layer(RateLimitLayer::new(ARGS.max_requests, Duration::from_secs(ARGS.reset_interval)))
                .layer(TimeoutLayer::new(Duration::from_secs(ARGS.timeout))),
        );

    let listener = TcpListener::bind(&ARGS.endpoint)
        .await
        .context(format!("Failed to bind to {}", ARGS.endpoint))?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .context("File server failed")?;

    Ok(())
}

async fn revision_checker() -> anyhow::Result<()> {
    loop {
        let patch_info = PatchInfo::fetch_latest(&ARGS.patch_host, &ARGS.patch_port)
            .await
            .context("Failed to fetch latest patch info")?;

        let mut fetcher = AssetFetcher::new(&patch_info, ARGS.concurrent_downloads, &ARGS.save_directory).unwrap();
        fetcher
            .fetch_manifest()
            .await
            .context("Failed to fetch and process manifest")?;

        let assets = fetcher.assets.clone();
        let new_revision =
            LocalRevision::new(&patch_info.revision, &ARGS.save_directory, assets).context("Failed to create local revision")?;
        let latest_revision = LocalRevision::latest().await;

        if let Ok(diff) = compare_revisions(&new_revision, latest_revision).await {
            println!("[INFO] Revision found: {}", new_revision.name);
            REVISIONS
                .write()
                .await
                .insert(new_revision.clone());

            if !diff.new_assets.is_empty() {
                println!("[INFO] fetching {} new assets...", diff.new_assets.len());
                fetcher
                    .download_assets(diff.new_assets.clone())
                    .await;
            }

            if !diff.changed_assets.is_empty() {
                println!("[INFO] fetching {} changed assets...", diff.changed_assets.len());
                fetcher
                    .download_assets(diff.changed_assets.clone())
                    .await;
            }

            cfg!(debug_assertions).then(|| {
                println!(
                    "New Assets: {}, Removed Assets: {}, Changed Assets: {}, Unchanged Assets: {}",
                    diff.new_assets.len(),
                    diff.removed_assets.len(),
                    diff.changed_assets.len(),
                    diff.unchanged_assets.len()
                );
            });
        }

        sleep(Duration::from_secs(ARGS.fetch_interval)).await;
    }
}
