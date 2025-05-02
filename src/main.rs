use clap::Parser;
use fetcher::client::AssetFetcher;
use revision::Revision;
use std::{
    error::Error,
    net::{Ipv4Addr, SocketAddrV4},
    num::NonZeroUsize,
    path::PathBuf,
};

pub mod errors;
pub mod extract;
pub mod fetcher;
pub mod models;
pub mod revision;
pub mod util;

#[derive(Clone, Parser)]
#[clap(author, version, about)]
struct Args {
    #[arg(short, long, env = "ENDPOINT", default_value_t = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 12369))]
    endpoint: SocketAddrV4,

    #[arg(short, long, env = "CONCURRENT_DOWNLOADS", default_value_t = unsafe { NonZeroUsize::new_unchecked(1) })]
    concurrent_downloads: NonZeroUsize,

    #[arg(short, long, env = "SAVE_DIRECTORY", default_value = "data")]
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

// pub async fn file(Path((revision, file_path)): Path<(String, String)>, ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
//     ()
// }

// pub async fn revisions(Path((revision, file_path)): Path<(String, String)>, ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
//     ()
// }
