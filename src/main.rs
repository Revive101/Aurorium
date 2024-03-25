use crate::{
    http::http_request::HttpRequest,
    routes::{get_revisions, get_util, get_wad, get_xml_filelist},
};
use axum::{routing::get, Extension, Router};
use bpaf::{construct, long, short, OptionParser, Parser};
use lazy_static::lazy_static;
use revision_checker::revision_checker::Revision;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};
use util::explore_revisions;

pub mod errors;
mod http;
mod rate_limit;
mod revision_checker;
mod routes;
mod util;

lazy_static! {
    pub static ref REVISIONS: RwLock<Vec<String>> = RwLock::new(vec![]);
    pub static ref LATEST_REVISION: RwLock<(String, String)> =
        RwLock::new((String::new(), String::new()));
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Opt {
    verbose: bool,
    ip: SocketAddr,
    concurrent_downloads: usize,
    rl_max_requests: u32,
    rl_reset_duration: u32,
    rl_disable: bool,
    rc_interval: u64,
}

fn opts() -> OptionParser<Opt> {
    let verbose = short('v')
        .long("verbose")
        .help("Activate verbosity (Default: warn)")
        .switch();

    let ip = short('i')
        .long("ip")
        .help("Override the default endpoint IP (Default: 0.0.0.0:12369)")
        .argument::<SocketAddr>("SocketAddr")
        .fallback("0.0.0.0:12369".parse().unwrap());

    let concurrent_downloads = short('c')
        .long("concurrent_downloads")
        .help("Override the count of concurrent downloads at once (Default: 8)")
        .argument::<usize>("usize")
        .fallback(8);

    let rl_max_requests = long("max_requests").
        help("Change the amount of requests a user can send before getting rate-limited by the server (Default: 100)").
        argument::<u32>("u32").
        fallback(100);

    let rl_reset_duration = long("reset_duration")
        .help("Change the duration for the interval in which the rate-limit list get's cleared (In seconds) (Default: 60)")
        .argument::<u32>("u32")
        .fallback(60);

    let rl_disable = long("disable_ratelimit").help("Disable ratelimit").switch();

    let rc_interval = long("revision_check_interval")
        .help("Change the interval for checking for new revisions (In minutes)  (Default: 0)")
        .argument::<u64>("u64")
        .fallback(0);

    construct!(Opt { verbose, ip, concurrent_downloads, rl_max_requests, rl_reset_duration, rl_disable, rc_interval })
        .to_options()
        .footer("Copyright (c) 2023 Phill030")
        .descr("This project is not associated with Wizard101rewritten in any way. Any use of this in reference of Wizard101rewritten will not be tolerated.")
}

#[tokio::main]
async fn main() {
    let opts = opts().run();

    let filter = if opts.verbose { "info" } else { "warn" };
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(filter));

    check_revision(opts.concurrent_downloads).await;
    if opts.rc_interval > 0 {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60 * opts.rc_interval)).await;
                check_revision(opts.concurrent_downloads).await;
            }
        });
    }

    let state = Arc::new(Mutex::new(rate_limit::rate_limiter::RateLimiter::new(
        opts.rl_max_requests,
        Duration::from_secs(u64::from(opts.rl_reset_duration)),
        opts.rl_disable,
    )));

    // Initialize all routes
    let app = Router::new()
        .route("/patcher/revisions", get(get_revisions))
        .route("/patcher/:revision/wads/:filename", get(get_wad))
        .route("/patcher/:revision", get(get_xml_filelist))
        .route("/patcher/:revision/utils/:filename", get(get_util))
        .layer(Extension(state));

    log::info!("Starting HTTP server @ {}", &opts.ip);
    match axum::Server::bind(&opts.ip)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
    {
        Ok(_) => (),
        Err(why) => log::error!("Could not start Axum server! {}", why),
    }
}

async fn check_revision(concurrent_downloads: usize) {
    let fetched_revision = Revision::check().await.unwrap();
    if explore_revisions().await.is_err()
        || !REVISIONS
            .read()
            .unwrap()
            .to_vec()
            .contains(&fetched_revision.revision)
    {
        let mut req = HttpRequest::new(fetched_revision, concurrent_downloads);
        req.propogate_filelist().await;

        explore_revisions().await.unwrap();
    } else {
        log::info!("Newest revision is already fetched!");
    }
}
