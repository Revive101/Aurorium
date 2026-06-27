use crate::{ARGS, REVISIONS, models::revision::LocalRevision, utils::ConnectionAddr};
use axum::{
    BoxError,
    body::Body,
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::{AppendHeaders, IntoResponse},
};
use reqwest::header;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;

pub async fn file(
    Path((revision, file_path)): Path<(String, String)>,
    headers_map: HeaderMap,
    ConnectionAddr(addr): ConnectionAddr,
) -> impl IntoResponse {
    println!("{addr} connected to /{revision}/{file_path}");

    if let Some(revision_for_asset) = LocalRevision::find_revision_for_asset(revision, &file_path).await {
        let path = std::env::current_dir()
            .unwrap()
            .join(&ARGS.save_directory)
            .join(revision_for_asset)
            .join(&file_path);

        let mut file = match tokio::fs::File::open(&path).await {
            Ok(f) => f,
            Err(_) => return Err((StatusCode::NOT_FOUND, format!("File not found: {path:?}")).into_response()),
        };

        let file_length = file
            .metadata()
            .await
            .map(|meta| meta.len())
            .unwrap_or(0);

        let file_name = path
            .file_name()
            .unwrap()
            .to_string_lossy();

        // Parse Range header for partial requests
        if let Some(range_str) = headers_map.get("range").and_then(|v| v.to_str().ok()) {
            if let Some(range) = parse_range(range_str, file_length) {
                file.seek(SeekFrom::Start(range.start)).await.unwrap();
                let length = range.end - range.start + 1;
                let limited = file.take(length);

                let content_range = format!(
                    "bytes {}-{}/{}",
                    range.start, range.end, file_length
                );
                let headers = AppendHeaders([
                    (header::CONTENT_TYPE, "application/octet-stream"),
                    (header::CONTENT_LENGTH, &length.to_string()),
                    (header::CONTENT_RANGE, &content_range),
                    (header::ACCEPT_RANGES, "bytes"),
                ]);

                let stream = ReaderStream::new(limited);
                let body = Body::from_stream(stream);
                return Ok((StatusCode::PARTIAL_CONTENT, headers, body).into_response());
            }
        }

        // Full file response
        let header_content = format!("attachment; filename=\"{file_name}\"");
        let headers = AppendHeaders([
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CONTENT_DISPOSITION, &header_content),
            (header::CONTENT_LENGTH, &file_length.to_string()),
            (header::ACCEPT_RANGES, "bytes"),
        ]);

        let stream = ReaderStream::new(file);
        let body = Body::from_stream(stream);
        Ok((headers, body).into_response())
    } else {
        Err((StatusCode::NOT_FOUND, format!("File not found: {file_path}")).into_response())
    }
}

struct ByteRange {
    start: u64,
    end: u64,
}

fn parse_range(range_str: &str, file_length: u64) -> Option<ByteRange> {
    let stripped = range_str.strip_prefix("bytes=")?;
    let (start_str, end_str) = stripped.split_once('-')?;
    let start: u64 = start_str.parse().ok()?;
    let end: u64 = if end_str.is_empty() {
        file_length - 1
    } else {
        end_str.parse().ok()?
    };

    if start >= file_length || end >= file_length || start > end {
        return None;
    }

    Some(ByteRange { start, end })
}

pub async fn revisions(ConnectionAddr(addr): ConnectionAddr) -> impl IntoResponse {
    println!("{addr} connected to /revisions");

    let revisions = REVISIONS
        .read()
        .await
        .iter()
        .map(|r| r.name.clone())
        .collect::<Vec<_>>();
    let headers = AppendHeaders([(header::CONTENT_TYPE, "application/json; charset=utf-8")]);

    (headers, json!(*revisions).to_string()).into_response()
}

pub async fn handle_error(error: BoxError) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("Unhandled error: {error}"))
}
