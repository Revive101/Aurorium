use crate::{AppState, utils::ConnectionAddr};
use axum::{
    extract::State,
    response::{AppendHeaders, IntoResponse},
};
use reqwest::header;
use serde_json::json;
use tracing::debug;

pub async fn get_latest_revision(
    State(state): State<AppState>,
    ConnectionAddr(addr): ConnectionAddr,
) -> impl IntoResponse {
    debug!("GET /latest from {}", addr);

    let latest_revision = state.db.latest_revision().await.unwrap();
    let headers = AppendHeaders([(header::CONTENT_TYPE, "application/json; charset=utf-8")]);

    let content = match latest_revision {
        Some(revision) => revision.name,
        None => json!({}).to_string(),
    };

    (headers, content).into_response()
}
