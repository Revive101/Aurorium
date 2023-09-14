use axum::{
    body::StreamBody,
    extract::{ConnectInfo, Path},
    headers::UserAgent,
    response::{AppendHeaders, IntoResponse},
    TypedHeader,
};
use reqwest::{header, StatusCode};
use serde_json::json;
use std::net::SocketAddr;
use tokio_util::io::ReaderStream;

use crate::{util::log_access, REVISIONS};

pub async fn get_revisions(
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_access(addr, &user_agent, "/patcher/revisions");

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

pub async fn get_wad(
    Path((revision, filename)): Path<(String, String)>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_access(
        addr,
        &user_agent,
        &format!("/patcher/{revision}/wad/{filename}"),
    );

    let path = format!("files/{revision}/wads/{filename}");

    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) => {
            return Err((StatusCode::NOT_FOUND, format!("File not found: {err}")).into_response())
        }
    };

    let file_length = file
        .metadata()
        .await
        .map(|meta| meta.len())
        .unwrap_or(0)
        .to_string();

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    let header_content = format!("attachment; filename=\"{filename}\"");
    let headers = AppendHeaders([
        (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
        (header::CONTENT_DISPOSITION, &header_content),
        (header::CONTENT_LENGTH, &file_length.as_str()),
    ]);

    Ok((headers, body).into_response())
}

pub async fn get_xml_filelist(
    Path(revision): Path<String>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_access(addr, &user_agent, &format!("/patcher/{}", revision));

    let path = format!("files/{revision}/LatestFileList.xml");

    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) => {
            return Err((StatusCode::NOT_FOUND, format!("File not found: {err}")).into_response())
        }
    };

    let file_length = file
        .metadata()
        .await
        .map(|meta| meta.len())
        .unwrap_or(0)
        .to_string();

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    let headers = AppendHeaders([
        (header::CONTENT_TYPE, "text/xml; charset=utf-8"),
        (
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"LatestFileList.xml\"",
        ),
        (header::CONTENT_LENGTH, file_length.as_str()),
    ]);

    Ok((headers, body).into_response())
}

pub async fn get_util(
    Path((revision, filename)): Path<(String, String)>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_access(
        addr,
        &user_agent,
        &format!("/patcher/{revision}/utils/{filename}"),
    );

    let path = format!("files/{revision}/utils/{filename}");

    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) => {
            return Err((StatusCode::NOT_FOUND, format!("File not found: {err}")).into_response())
        }
    };

    let file_length = file
        .metadata()
        .await
        .map(|meta| meta.len())
        .unwrap_or(0)
        .to_string();

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    let header_content = format!("attachment; filename=\"{filename}\"");
    let headers = AppendHeaders([
        (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
        (header::CONTENT_DISPOSITION, &header_content),
        (header::CONTENT_LENGTH, file_length.as_str()),
    ]);

    Ok((headers, body).into_response())
}
