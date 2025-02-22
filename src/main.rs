use axum::{
    BoxError, Router, error_handling::HandleErrorLayer, response::IntoResponse, routing::get,
};
use clap::Parser;
use lazy_static::lazy_static;
use reqwest::StatusCode;
use revision_checker::checker::RevisionChecker;
use routes::{get_file, get_revisions};
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    process::exit,
    time::Duration,
};
use tokio::{join, net::TcpListener, spawn, sync::RwLock, time::sleep};
use tower::{ServiceBuilder, buffer::BufferLayer, limit::RateLimitLayer, timeout::TimeoutLayer};

pub mod errors;
mod fetcher;
mod revision_checker;
mod routes;
pub mod util;

lazy_static! {
    pub static ref REVISIONS: RwLock<Vec<String>> = RwLock::new(Vec::new());
}

#[derive(Clone, Parser)]
#[command(name = "Aurorium")]
#[command(bin_name = "aurorium")]
#[command(about = "File & Backendserver for files associated with the Wizard101 client", long_about = None)]
#[command(version = "2.0")]
struct Arguments {
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    #[arg(short, long, default_value_t = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 12369))]
    /// Socket address to listen
    endpoint: SocketAddrV4,

    #[arg(short, long, default_value_t = 16)]
    /// Max number of concurrent downloads while fetching
    concurrent_downloads: usize,

    #[arg(short, long, default_value_t = 0)]
    /// Interval in minutes to check for a new revision
    fetch_interval: u64,

    #[arg(short, long, default_value_t = 100)]
    /// Max requests in the RateLimiter until connections getting blocked by the system
    max_requests: u64,

    #[arg(short, long, default_value_t = 60)]
    /// Interval in seconds in the RateLimiter for when to clear the RateLimit list
    reset_interval: u64,

    #[arg(short, long, default_value_t = 10)]
    /// Lets each connection timeout after x seconds of no response
    timeout: u64,
}

#[tokio::main]
async fn main() {
    let cli = Arguments::parse();

    let filter = if cli.verbose { "info" } else { "warn" };
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(filter));

    let axum_task = spawn(async move {
        let router = Router::new()
            .route("/:revision/*file_path", get(get_file))
            .route("/revisions", get(get_revisions))
            .layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(handle_error))
                    .layer(BufferLayer::new(1024))
                    .layer(RateLimitLayer::new(
                        cli.max_requests,
                        Duration::from_secs(cli.reset_interval),
                    ))
                    .layer(TimeoutLayer::new(Duration::from_secs(cli.timeout))),
            );

        let listener = TcpListener::bind(&cli.endpoint).await.unwrap();
        if let Err(why) = axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        {
            log::error!("Failed to serve axum server: {why}");
            exit(1);
        }
    });

    let mut checker = RevisionChecker::new(cli.concurrent_downloads)
        .await
        .unwrap();
    let revision_task = tokio::spawn(async move {
        if cli.fetch_interval > 0 {
            loop {
                checker.check_latest_revision_fetched().await;
                sleep(Duration::from_secs(60 * cli.fetch_interval)).await;
            }
        }
    });

    let (_, _) = join!(axum_task, revision_task);
}

async fn handle_error(error: BoxError) -> impl IntoResponse {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Unhandled error: {}", error),
    )
}
