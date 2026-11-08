use crate::{AppState, errors::RouteError, utils::ConnectionAddr};
use axum::{Json, extract::State, response::IntoResponse};
use tracing::debug;

pub async fn get_latest_revision(
    State(state): State<AppState>,
    ConnectionAddr(addr): ConnectionAddr,
) -> Result<impl IntoResponse, RouteError> {
    debug!("GET /latest from {}", addr);

    let latest_revision = state.db.latest_revision().await?;

    Ok(Json(latest_revision.map(|revision| revision.name)))
}
