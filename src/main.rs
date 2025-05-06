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
use tokio::{join, net::TcpListener, sync::RwLock, time::sleep};
use tower::{ServiceBuilder, buffer::BufferLayer, limit::RateLimitLayer, timeout::TimeoutLayer};

pub mod errors;
pub mod fetcher;
pub mod models;
pub mod patch_info;
mod routes;
pub mod utils;
pub mod xml_parser;

const HOST: &str = "patch.us.wizard101.com";
const PORT: &str = "12500";

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

    #[arg(long, env = "HOST", default_value = HOST)]
    host: String,

    #[arg(long, env = "PORT", default_value = PORT)]
    port: String,

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
async fn main() -> std::io::Result<()> {
    // Initialize all revisions on disk
    LocalRevision::init_all(&ARGS.save_directory).await?;

    // Start file server
    let file_serving = tokio::spawn(async move {
        let router = Router::new()
            .route("/{revision}/{*file_path}", get(file))
            .route("/revisions", get(revisions))
            .layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(handle_error))
                    .layer(BufferLayer::new(1024))
                    .layer(RateLimitLayer::new(ARGS.max_requests, Duration::from_secs(ARGS.reset_interval)))
                    .layer(TimeoutLayer::new(Duration::from_secs(ARGS.timeout))),
            );

        let listener = TcpListener::bind(&ARGS.endpoint).await.unwrap();
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();
    });

    // Periodically check for new revisions
    let revision_checker = tokio::spawn(async move {
        loop {
            let patch_info = PatchInfo::fetch_latest(&ARGS.host, &ARGS.port).await.unwrap();

            let mut asset_fetcher = AssetFetcher::new(&patch_info, ARGS.concurrent_downloads, &ARGS.save_directory);
            asset_fetcher.fetch_index().await.unwrap();

            let assets = asset_fetcher.assets.clone();
            let new_rev = LocalRevision::new(&patch_info.revision, &ARGS.save_directory, assets).unwrap();

            let newest_rev_on_disk = LocalRevision::newest().await;

            if let Ok(compared) = compare_revisions(&new_rev, newest_rev_on_disk).await {
                println!("[INFO] New revision found: {}", new_rev.name);
                REVISIONS.write().await.insert(new_rev.clone());

                if !compared.new_assets.is_empty() {
                    println!("[INFO] fetching new assets...");
                    asset_fetcher.fetch_files(compared.new_assets.clone()).await;
                }

                if !compared.changed_assets.is_empty() {
                    println!("[INFO] fetching changed assets...");
                    asset_fetcher.fetch_files(compared.changed_assets.clone()).await;
                }

                cfg!(debug_assertions).then(|| {
                    println!(
                        "New Assets: {}, Removed Assets: {}, Changed Assets: {}, Unchanged Assets: {}",
                        compared.new_assets.len(),
                        compared.removed_assets.len(),
                        compared.changed_assets.len(),
                        compared.unchanged_assets.len()
                    );
                });
            }

            sleep(Duration::from_secs(ARGS.fetch_interval)).await;
        }
    });

    let (_, _) = join!(file_serving, revision_checker);
    Ok(())
}
