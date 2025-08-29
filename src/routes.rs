use crate::{ARGS, BACKUP_FILES, REVISIONS, models::revision::LocalRevision, utils::ConnectionAddr};
use axum::{
    BoxError,
    body::Body,
    extract::Path,
    response::{
        AppendHeaders, IntoResponse, Sse,
        sse::{Event, KeepAlive},
    },
};
use reqwest::{StatusCode, header};
use serde_json::json;
use std::{
    convert::Infallible,
    net::{Ipv4Addr, SocketAddrV4},
    sync::LazyLock,
};
use tokio::{sync::RwLock, time::Instant};
use tokio_stream::{StreamExt, wrappers::IntervalStream};
use tokio_util::io::ReaderStream;

pub async fn file(Path((revision, file_path)): Path<(String, String)>, ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
    println!("{addr} connected to /{revision}/{file_path}");

    if let Some(revision_for_asset) = LocalRevision::find_revision_for_asset(revision, &file_path).await {
        let path = ARGS.save_directory.join(revision_for_asset).join(file_path);
        let Ok(file) = tokio::fs::File::open(&path).await else {
            return Err((StatusCode::NOT_FOUND, format!("File not found: {path:?}")).into_response());
        };

        let file_length = file.metadata().await.map(|meta| meta.len()).unwrap_or(0).to_string();
        let file_name = path.file_name().unwrap().to_string_lossy();
        let header_content = format!("attachment; filename=\"{file_name}\"");
        let headers = AppendHeaders([
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CONTENT_DISPOSITION, &header_content),
            (header::CONTENT_LENGTH, file_length.as_str()),
        ]);

        let stream = ReaderStream::new(file);
        let body = Body::from_stream(stream);

        Ok((headers, body).into_response())
    } else {
        Err((StatusCode::NOT_FOUND, format!("File not found: {file_path}")).into_response())
    }
}

pub async fn revisions(ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
    println!("{addr} connected to /revisions");

    let revisions = REVISIONS.read().await.iter().map(|r| r.name.clone()).collect::<Vec<_>>();
    let headers = AppendHeaders([(header::CONTENT_TYPE, "application/json; charset=utf-8")]);

    (headers, json!(*revisions).to_string()).into_response()
}

pub async fn handle_error(error: BoxError) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("Unhandled error: {error}"))
}

// Mirror

pub async fn mirror_file(Path((revision, file_path)): Path<(String, String)>, ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
    println!("{addr} connected to /mirror/files/{revision}/{file_path}");

    let Ok(ip) = addr.parse::<Ipv4Addr>() else {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid or unsupported IP address (Only IPv4 addresses are supported)",
        )
            .into_response());
    };

    if !ARGS.mirror_ips.contains(&ip) {
        return Err((StatusCode::FORBIDDEN, "IP address not allowed").into_response());
    }

    let path = ARGS.save_directory.join(revision).join(file_path);
    let Ok(file) = tokio::fs::File::open(&path).await else {
        return Err((StatusCode::NOT_FOUND, format!("File not found: {path:?}")).into_response());
    };

    let file_length = file.metadata().await.map(|meta| meta.len()).unwrap_or(0).to_string();
    let file_name = path.file_name().unwrap().to_string_lossy();
    let header_content = format!("attachment; filename=\"{file_name}\"");
    let headers = AppendHeaders([
        (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
        (header::CONTENT_DISPOSITION, &header_content),
        (header::CONTENT_LENGTH, file_length.as_str()),
    ]);

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok((headers, body).into_response())
}

pub async fn backup_sse(ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
    println!("{addr} connected to /mirror");
    let Ok(ip) = addr.parse::<Ipv4Addr>() else {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid or unsupported IP address (Only IPv4 addresses are supported)",
        )
            .into_response());
    };

    if !ARGS.mirror_ips.contains(&ip) {
        return Err((StatusCode::FORBIDDEN, "IP address not allowed").into_response());
    }

    let stream =
        IntervalStream::new(tokio::time::interval(std::time::Duration::from_secs(ARGS.broadcast_interval))).then(move |_| async move {
            let files = BACKUP_FILES.read().await.clone();
            Ok::<_, Infallible>(Event::default().data(json!(files).to_string()))
        });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

// Server discovery

pub struct BackupNode {
    pub addr: SocketAddrV4,
    pub last_seen: Instant,
}
pub static BACKUP_NODES: LazyLock<RwLock<Vec<BackupNode>>> = LazyLock::new(|| RwLock::new(Vec::new()));

pub async fn discover(ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
    todo!()
}

pub async fn heartbeat(ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
    todo!()
}
