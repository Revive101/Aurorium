use crate::{AppState, errors::RouteError, utils::ConnectionAddr};
use axum::{Json, extract::State, response::IntoResponse};
use tracing::debug;

pub async fn get_revisions(
    State(state): State<AppState>,
    ConnectionAddr(addr): ConnectionAddr,
) -> Result<impl IntoResponse, RouteError> {
    debug!("GET /revisions from {}", addr);

    let revisions = state.db.list_revisions().await?;

    Ok(Json(revisions))
}
