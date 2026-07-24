use crate::{AppState, utils::ConnectionAddr};
use axum::{
    extract::State,
    response::{AppendHeaders, IntoResponse},
};
use reqwest::header;
use serde_json::json;
use tracing::debug;

pub async fn get_revisions(
    State(state): State<AppState>,
    ConnectionAddr(addr): ConnectionAddr,
) -> impl IntoResponse {
    debug!("GET /revisions from {}", addr);

    let revisions = state.db.list_revisions().await.unwrap_or(vec![]);
    let headers = AppendHeaders([(header::CONTENT_TYPE, "application/json; charset=utf-8")]);

    (headers, json!(revisions).to_string()).into_response()
}
