/*
    Copyright (c) 2023 Phill030. All rights reserved.
    This code is exclusive to Revive101.

    Unauthorized use, reproduction, or distribution of this code,
    in whole or in part, by any party outside of Revive101 is prohibited.
*/
use axum::{
    body::StreamBody,
    extract::{ConnectInfo, Path},
    headers::UserAgent,
    http::header,
    response::{AppendHeaders, IntoResponse},
    routing::get,
    Router, TypedHeader,
};
use bpaf::{construct, short, OptionParser, Parser};
use lazy_static::lazy_static;
use reqwest::StatusCode;
use serde_json::json;
use std::{net::SocketAddr, process, sync::Mutex};
use tokio_util::io::ReaderStream;
use util::{explore_revisions, log_access};

mod http;
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
        .descr("By default, only the webserver will get started. If you want to try to fetch from a revision, use the --revision or -r parameter.")
}

#[tokio::main]
async fn main() {
    let opts = opts().run();

    // File Logging
    // let formatted_time = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    //
    // let target = Box::new(
    // std::fs::File::create(format!("{formatted_time}_server.log")).expect("Can't create file"),
    // );
    // env_logger::Builder::new()
    //     .target(env_logger::Target::Pipe(target))
    //     .filter(None, log::LevelFilter::Info)
    //     .init();

    let filter = if opts.verbose { "info" } else { "warn" };
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(filter));

    if opts.revision.is_some() {
        //todo: TOKIO NOT NEEDED ANYMORE??????? 🤩😳😲 TESTING BEGINS SOON??????
        let mut req =
            http::http_request::HttpRequest::new(opts.revision.unwrap(), opts.concurrent_downloads)
                .await;
        req.propogate_filelist().await;
    }

    // If there are no files to host, why have the server running anyways? 🤓☝
    match explore_revisions().await {
        Ok(_) => {}
        Err(_) => {
            log::error!("There are no revisions for the server to host! (Quitting)");
            process::exit(0);
        }
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

async fn get_revisions(
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_access(addr, user_agent, "/patcher/revisions".to_string());

    let folders = match REVISIONS.lock() {
        Ok(r) => r.clone(),
        Err(why) => {
            log::error!("Could not lock REVISIONS, {why}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "500 - Internal Error!".to_string(),
            )
                .into_response());
        }
    };

    let headers = AppendHeaders([(header::CONTENT_TYPE, "application/json; charset=utf-8")]);

    Ok((headers, json!(folders).to_string()).into_response())
}

async fn get_wad(
    Path((revision, filename)): Path<(String, String)>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_access(
        addr,
        user_agent,
        format!("/patcher/{}/wad/{}", revision, filename),
    );

    let path = format!("files/{revision}/wads/{filename}");

    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) => {
            return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err)).into_response())
        }
    };

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    let header_content = format!("attachment; filename=\"{}\"", filename);
    let headers = AppendHeaders([
        (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
        (header::CONTENT_DISPOSITION, &header_content),
    ]);

    Ok((headers, body).into_response())
}

async fn get_xml_filelist(
    Path(revision): Path<String>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_access(addr, user_agent, format!("/patcher/{}", revision));

    let path = format!("files/{revision}/LatestFileList.xml");

    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) => {
            return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err)).into_response())
        }
    };

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    let headers = AppendHeaders([
        (header::CONTENT_TYPE, "text/xml; charset=utf-8"),
        (
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"LatestFileList.xml\"",
        ),
    ]);

    Ok((headers, body).into_response())
}

async fn get_util(
    Path((revision, filename)): Path<(String, String)>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_access(
        addr,
        user_agent,
        format!("/patcher/{}/utils/{}", revision, filename),
    );

    let path = format!("files/{revision}/utils/{filename}");

    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) => {
            return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err)).into_response())
        }
    };

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    let header_content = format!("attachment; filename=\"{}\"", filename);
    let headers = AppendHeaders([
        (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
        (header::CONTENT_DISPOSITION, &header_content),
    ]);

    Ok((headers, body).into_response())
}
