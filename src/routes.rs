use crate::{REVISIONS, util::log_access};
use axum::{
    body::Body,
    extract::{FromRequestParts, Path},
    http::request::Parts,
    response::{AppendHeaders, IntoResponse},
};
use axum_extra::{TypedHeader, headers::UserAgent};
use reqwest::{StatusCode, header};
use serde_json::json;
use std::{convert::Infallible, path::PathBuf};
use tokio_util::io::ReaderStream;

#[derive(Debug)]
pub struct XForwardedFor(pub String);

impl<S> FromRequestParts<S> for XForwardedFor
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> std::result::Result<Self, Self::Rejection> {
        parts
            .headers
            .get("X-Forwarded-For")
            .and_then(|value| value.to_str().ok())
            .map(|s| XForwardedFor(s.to_string()))
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

pub async fn get_file(
    Path((revision, file_path)): Path<(String, String)>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    XForwardedFor(addr): XForwardedFor,
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
    XForwardedFor(addr): XForwardedFor,
) -> Result<impl IntoResponse, Infallible> {
    log_access(addr, &user_agent, "/revisions");

    let revisions = REVISIONS.read().await;
    let headers = AppendHeaders([(header::CONTENT_TYPE, "application/json; charset=utf-8")]);

    Ok((headers, json!(*revisions).to_string()).into_response())
}
