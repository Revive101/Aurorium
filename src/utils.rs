use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use std::net::SocketAddr;

use crate::fetcher::compare::RevisionDiff;

pub enum Endianness {
    Little,
    Big,
}

pub fn hex_decode(hex_string: &str, endianness: &Endianness) -> Option<Vec<u8>> {
    if hex_string.len() % 2 != 0 {
        return None;
    }

    let bytes: Option<Vec<u8>> = (0..hex_string.len())
        .step_by(2)
        .map(|i| {
            let byte = u8::from_str_radix(&hex_string[i..i + 2], 16).ok()?;
            match endianness {
                Endianness::Little => Some(byte),
                Endianness::Big => Some(byte.reverse_bits()),
            }
        })
        .collect();

    bytes
}

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

pub fn format_new_assets(diff: &RevisionDiff) -> String {
    if diff.new_assets.is_empty() {
        return String::from("No new assets.");
    }

    let mut message = String::from("```ansi\n[2;32m");
    for asset in &diff.new_assets {
        message.push_str(&format!("{}\n", asset.filename));
    }
    message.push_str("[0m```");
    message
}

pub fn format_changed_assets(diff: &RevisionDiff) -> String {
    if diff.changed_assets.is_empty() {
        return String::from("No changed assets.");
    }

    let mut message = String::from("```ansi\n[2;32m[2;33m[0m[2;32m[2;33m");
    for asset in &diff.changed_assets {
        message.push_str(&format!("{}\n", asset.filename));
    }
    message.push_str("[0m[2;32m[0m```");
    message
}

pub fn format_removed_assets(diff: &RevisionDiff) -> String {
    if diff.removed_assets.is_empty() {
        return String::from("No removed assets.");
    }

    let mut message = String::from("```ansi\n[2;32m[2;33m[0m[2;32m[2;33m[2;31m");
    for asset in &diff.removed_assets {
        message.push_str(&format!("{}\n", asset.filename));
    }
    message.push_str("[0m[2;33m[0m[2;32m[0m```");
    message
}
