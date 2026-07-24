use crate::{AppState, utils::ConnectionAddr};
use axum::{
    extract::{Path, Request, State},
    response::IntoResponse,
};
use reqwest::StatusCode;
use std::path::{Component, Path as StdPath};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::{debug, warn};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum RouteError {
    #[error("File not found: {0}")]
    #[diagnostic(
        code(route::file_not_found),
        help("Ensure the requested file exists on the server.")
    )]
    NotFound(String),

    #[error("Database error: {0}")]
    #[diagnostic(
        code(route::database_error),
        help("Check the database connection and query for correctness.")
    )]
    Database(#[from] crate::db::DbError),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Invalid working directory")]
    #[diagnostic(
        code(route::invalid_working_dir),
        help("The server's working directory is invalid. Please check the server configuration.")
    )]
    InvalidWorkingDir(#[from] std::io::Error),
}

impl IntoResponse for RouteError {
    fn into_response(self) -> axum::response::Response {
        match self {
            RouteError::NotFound(file) => {
                (StatusCode::NOT_FOUND, format!("File not found: {file}")).into_response()
            }
            RouteError::Database(err) => {
                warn!(error = %err, "Database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
            RouteError::InvalidWorkingDir(dir) => {
                tracing::error!(error = %dir, "Invalid working directory");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Server misconfiguration: Invalid working directory",
                )
                    .into_response()
            }
            RouteError::BadRequest(file_path) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid file path: {file_path}"),
            )
                .into_response(),
        }
    }
}

pub async fn file(
    State(state): State<AppState>,
    Path((revision, file_path)): Path<(String, String)>,
    ConnectionAddr(addr): ConnectionAddr,
    req: Request,
) -> Result<impl IntoResponse, RouteError> {
    debug!("GET /{revision}/{file_path} from {addr}");

    // Prevent directory traversal. Better to be safe than sorry
    if StdPath::new(&file_path).components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(RouteError::BadRequest(file_path));
    }

    // If the file is LatestFileList.xml or LatestFileList.bin, we know that it belongs to the current revision
    let revision_for_asset = if file_path.contains("LatestFileList") {
        revision
    } else {
        state
            .db
            .get_revision_for_asset(revision, file_path.clone())
            .await?
            .ok_or_else(|| RouteError::NotFound(file_path.clone()))?
    };

    let path = std::env::current_dir()?
        .join(&state.config.fetcher.save_directory)
        .join(revision_for_asset)
        .join(file_path);

    match ServeFile::new(path).oneshot(req).await {
        Ok(res) => Ok(res.into_response()),
        Err(err) => match err {},
    }
}
