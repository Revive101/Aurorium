use crate::{util::log_access, REVISIONS};
use axum::{
    body::Body,
    extract::{ConnectInfo, Path},
    response::{AppendHeaders, IntoResponse},
};
use axum_extra::{headers::UserAgent, TypedHeader};
use reqwest::{header, StatusCode};
use serde_json::json;
use std::{convert::Infallible, net::SocketAddr, path::PathBuf};
use tokio_util::io::ReaderStream;

pub async fn get_file(
    Path((revision, file_path)): Path<(String, String)>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_access(addr, &user_agent, &format!("/files/{revision}/{file_path}"));

    let path = PathBuf::from("files").join(revision).join(file_path);
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(_) => return Err((StatusCode::NOT_FOUND, format!("File not found: {path:?}")).into_response()),
    };

    let file_length = file.metadata().await.map(|meta| meta.len()).unwrap_or(0).to_string();
    let file_name = path.file_name().unwrap().to_string_lossy();
    let header_content = format!("attachment; filename=\"{}\"", file_name.to_string());
    let headers = AppendHeaders([
        (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
        (header::CONTENT_DISPOSITION, &header_content),
        (header::CONTENT_LENGTH, file_length.as_str()),
    ]);

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok((headers, body).into_response())
}

pub async fn get_revisions(
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, Infallible> {
    log_access(addr, &user_agent, "/revisions");

    let revisions = REVISIONS.read().await;
    let headers = AppendHeaders([(header::CONTENT_TYPE, "application/json; charset=utf-8")]);

    Ok((headers, json!(*revisions).to_string()).into_response())
}
