use clap::Parser;
use fetcher::compare::compare_revisions;
use models::revision::LocalRevision;
use patch_info::PatchInfo;
use std::{
    error::Error,
    net::{Ipv4Addr, SocketAddrV4},
    num::NonZeroUsize,
    path::PathBuf,
};

pub mod errors;
pub mod fetcher;
pub mod models;
pub mod patch_info;
pub mod utils;
pub mod xml_parser;

const HOST: &str = "patch.us.wizard101.com";
const PORT: &str = "12500";

#[derive(Clone, Parser)]
#[clap(author, version, about)]
struct Args {
    #[arg(short, long, env = "ENDPOINT", default_value_t = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 12369))]
    endpoint: SocketAddrV4,

    #[arg(short, long, env = "CONCURRENT_DOWNLOADS", default_value_t = unsafe { NonZeroUsize::new_unchecked(1) })]
    concurrent_downloads: NonZeroUsize,

    #[arg(short, long, env = "SAVE_DIRECTORY", default_value = "data")]
    save_directory: PathBuf,

    #[arg(long, env = "HOST", default_value = HOST)]
    host: String,

    #[arg(long, env = "PORT", default_value = PORT)]
    port: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let revision = PatchInfo::fetch_latest(&args.host, &args.port).await?;
    let new_rev = LocalRevision::new(&revision.revision, &args.save_directory).await.unwrap();
    let old_rev = LocalRevision::from_name("V_r759882.Wizard_1_550", &args.save_directory).await;

    let mut compared = compare_revisions(&new_rev, &old_rev)?;

    cfg!(debug_assertions).then(|| {
        println!(
            "New Assets: {}, Removed Assets: {}, Changed Assets: {}, Unchanged Assets: {}",
            compared.new_assets.len(),
            compared.removed_assets.len(),
            compared.changed_assets.len(),
            compared.unchanged_assets.len()
        );

        println!("Changed:");
        while let Some(changed) = compared.changed_assets.pop() {
            println!("{}", changed.filename);
        }

        println!("Removed:");
        while let Some(removed) = compared.removed_assets.pop() {
            println!("{:?}", removed);
        }
    });

    //-----

    // AssetFetcher::new(revision, args.concurrent_downloads, args.save_directory)
    // .fetch_index()
    // .await?;

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
