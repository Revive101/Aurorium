use std::{
    net::SocketAddr,
    process,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{routing::get, Extension, Router};
use bpaf::{construct, long, short, OptionParser, Parser};
use lazy_static::lazy_static;

use util::explore_revisions;

use crate::routes::{get_revisions, get_util, get_wad, get_xml_filelist};

mod http;
mod rate_limit;
mod routes;
mod util;

lazy_static! {
    pub static ref REVISIONS: Mutex<Vec<String>> = Mutex::new(vec![]);
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Opt {
    verbose: bool,
    revision: Option<String>,
    ip: SocketAddr,
    concurrent_downloads: usize,
    rl_max_requests: u32,
    rl_reset_duration: u32,
}

fn opts() -> OptionParser<Opt> {
    let verbose = short('v')
        .long("verbose")
        .help("Activate verbosity (Default: warn)")
        .switch();

    let revision = short('r')
        .long("revision")
        .help("Fetch from a revision string (Example V_r740872.Wizard_1_520)")
        .argument::<String>("String")
        .optional();

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

    let rl_max_requests = long("max_requests").help("Change the amount of requests a user can send before getting rate-limited by the server").argument::<u32>("u32").fallback(100);

    let rl_reset_duration = long("reset_duration")
        .help("Change the duration for the interval in which the rate-limit list get's cleared (In seconds)")
        .argument::<u32>("u32")
        .fallback(60);

    construct!(Opt { verbose, revision, ip, concurrent_downloads, rl_max_requests, rl_reset_duration })
        .to_options()
        .footer("Copyright (c) 2023 Phill030")
        .descr("By default, only the webserver will get started. If you want to fetch from a revision, use the --revision or -r parameter.")
}

#[tokio::main]
async fn main() {
    let opts = opts().run();

    let filter = if opts.verbose { "info" } else { "warn" };
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(filter));

    if opts.revision.is_some() {
        let mut req =
            http::http_request::HttpRequest::new(opts.revision.unwrap(), opts.concurrent_downloads)
                .await;
        req.propogate_filelist().await;
    }

    // If there are no files to host, why have the server running anyways? 🤓☝
    if (explore_revisions().await).is_err() {
        log::error!("There are no revisions for the server to host! (Quitting)");
        process::exit(0);
    }

    let state = Arc::new(Mutex::new(rate_limit::rate_limiter::RateLimiter::new(
        opts.rl_max_requests,
        Duration::from_secs(u64::from(opts.rl_reset_duration)),
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
