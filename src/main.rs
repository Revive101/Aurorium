use axum::{extract::Path, response::IntoResponse};
use clap::Parser;
use extract::ConnectionAddr;
use indicatif::{ProgressBar, ProgressStyle};
use revision::{Revision, fetcher::AssetFetcher};
use std::{
    error::Error,
    net::{Ipv4Addr, SocketAddrV4},
    num::NonZeroUsize,
    path::PathBuf,
};

pub mod errors;
pub mod extract;
pub mod revision;
pub mod util;

#[derive(Clone, Parser)]
struct Args {
    #[arg(short, long, default_value_t = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 12369))]
    endpoint: SocketAddrV4,

    #[arg(short, long, default_value_t = unsafe { NonZeroUsize::new_unchecked(1) })]
    concurrent_downloads: NonZeroUsize,

    #[arg(short, long, default_value = "data")]
    save_directory: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let revision = Revision::check().await.unwrap();
    AssetFetcher::new(revision, args.concurrent_downloads, args.save_directory)
        .fetch_index()
        .await?;

    // let file_serving = spawn(async move {
    //     let router = Router::new()
    //         .route("/{revision}/{*file_path}", get(file))
    //         .route("/revisions", get(revisions));

    //     let listener = TcpListener::bind(&args.endpoint).await.unwrap();
    //     if let Err(why) = axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>()).await {}
    // });

    Ok(())
}

pub async fn file(Path((revision, file_path)): Path<(String, String)>, ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
    ()
}

pub async fn revisions(Path((revision, file_path)): Path<(String, String)>, ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
    ()
}
