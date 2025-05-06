use crate::{ARGS, REVISIONS, models::revision::LocalRevision, utils::ConnectionAddr};
use axum::{
    BoxError,
    body::Body,
    extract::Path,
    response::{AppendHeaders, IntoResponse},
};
use reqwest::{StatusCode, header};
use serde_json::json;
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
