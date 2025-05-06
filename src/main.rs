use axum::{Router, routing::get};
use clap::Parser;
use fetcher::{
    client::AssetFetcher,
    compare::{RevisionDiffError, compare_revisions},
};
use lazy_static::lazy_static;
use models::revision::LocalRevision;
use patch_info::PatchInfo;
use routes::{file, revisions};
use serde_json::json;
use std::{
    error::Error,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    num::NonZeroUsize,
    path::PathBuf,
    time::Duration,
};
use tokio::{join, net::TcpListener, sync::RwLock, time::sleep};
use utils::{format_changed_assets, format_new_assets, format_removed_assets};

pub mod errors;
pub mod fetcher;
pub mod models;
pub mod patch_info;
mod routes;
pub mod utils;
pub mod xml_parser;

const HOST: &str = "patch.us.wizard101.com";
const PORT: &str = "12500";

lazy_static! {
    pub static ref REVISIONS: RwLock<Vec<LocalRevision>> = RwLock::new(Vec::new());
    pub static ref ARGS: Args = Args::parse();
}

#[derive(Clone, Parser)]
#[clap(author, version, about)]
pub struct Args {
    #[arg(short, long, env = "ENDPOINT", default_value_t = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 12369))]
    endpoint: SocketAddrV4,

    #[arg(short, long, env = "CONCURRENT_DOWNLOADS", default_value_t = unsafe { NonZeroUsize::new_unchecked(4) })]
    concurrent_downloads: NonZeroUsize,

    #[arg(short, long, env = "SAVE_DIRECTORY", default_value = "data")]
    save_directory: PathBuf,

    #[arg(long, env = "HOST", default_value = HOST)]
    host: String,

    #[arg(long, env = "PORT", default_value = PORT)]
    port: String,

    #[arg(long, env = "WEBHOOK_TOKEN")]
    webhook_token: Option<String>,

    #[arg(short, long, default_value_t = 60)]
    fetch_interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize all revisions on disk
    LocalRevision::init_all(&ARGS.save_directory).await?;

    let file_serving = tokio::spawn(async move {
        let router = Router::new()
            .route("/{revision}/{*file_path}", get(file))
            .route("/revisions", get(revisions));

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
            let new_rev = LocalRevision::new(&patch_info.revision, &ARGS.save_directory, assets)
                .await
                .unwrap();

            let newest_rev_on_disk = LocalRevision::newest().await;

            let compared = compare_revisions(&new_rev, newest_rev_on_disk).await;
            match compared {
                Ok(compared) => {
                    println!("[INFO] New revision found: {}", new_rev.name);

                    if !compared.new_assets.is_empty() {
                        println!("[INFO] fetching new assets...");
                        asset_fetcher.fetch_files(compared.new_assets.clone()).await;
                    }

                    if !compared.changed_assets.is_empty() {
                        println!("[INFO] fetching changed assets...");
                        asset_fetcher.fetch_files(compared.changed_assets.clone()).await;
                    }

                    if let Some(webhook) = &ARGS.webhook_token {
                        if REVISIONS.read().await.iter().any(|r| r.name == new_rev.name) {
                            continue;
                        }

                        let payload = json!({
                          "content": "",
                          "embeds": [
                            {
                              "title": "📦 New Revision Detected",
                              "description": format!("Revision: `{}`\n\nNew Assets:\n{}\n\nChanged Assets:\n{}\n\nRemoved Assets:\n{}", new_rev.name, format_new_assets(&compared), format_changed_assets(&compared), format_removed_assets(&compared)),
                              "color": 65351
                            }
                          ],
                        });

                        let client = reqwest::Client::new();
                        client.post(webhook).json(&payload).send().await.unwrap();
                    }

                    REVISIONS.write().await.push(new_rev.clone());

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

                _ => {}
            }

            sleep(Duration::from_secs(ARGS.fetch_interval)).await;
        }
    });

    let (_, _) = join!(file_serving, revision_checker);
    Ok(())
}
