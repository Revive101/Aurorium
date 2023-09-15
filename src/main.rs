use std::{net::SocketAddr, process, sync::Mutex};

use axum::{Router, routing::get};
use bpaf::{construct, OptionParser, Parser, short};
use lazy_static::lazy_static;

use util::explore_revisions;

use crate::routes::{get_revisions, get_util, get_wad, get_xml_filelist};

mod http;
mod routes;
mod util;
mod rate_limit;

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
}

fn opts() -> OptionParser<Opt> {
    let verbose = short('v')
        .long("verbose")
        .help("Activate verbosity (Default: warn)")
        .switch();

    let revision = short('r')
        .long("revision")
        .help("Fetch from a revision string (Example: V_r739602.Wizard_1_520_0_Live)")
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

    construct!(Opt { verbose, revision, ip, concurrent_downloads })
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

    // Initialize all routes
    let app = Router::new()
        .route("/patcher/revisions", get(get_revisions))
        .route("/patcher/:revision/wads/:filename", get(get_wad))
        .route("/patcher/:revision", get(get_xml_filelist))
        .route("/patcher/:revision/utils/:filename", get(get_util));

    log::info!("Starting HTTP server @ {}", &opts.ip);
    match axum::Server::bind(&opts.ip)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
    {
        Ok(_) => (),
        Err(why) => log::error!("Could not start Axum server! {}", why),
    }
}
