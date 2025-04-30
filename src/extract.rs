use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use std::net::SocketAddr;

#[derive(Debug)]
pub struct ConnectionAddr(pub String);

impl<S> FromRequestParts<S> for ConnectionAddr
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // First try to get X-Forwarded-For header
        if let Some(forwarded_for) = parts.headers.get("X-Forwarded-For").and_then(|value| value.to_str().ok()) {
            return Ok(ConnectionAddr(forwarded_for.to_string()));
        }

        // If header not found, fall back to connection info
        let connection_info = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(ConnectionAddr(connection_info.0.ip().to_string()))
    }
}
