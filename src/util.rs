use std::net::SocketAddr;

use axum::headers::UserAgent;
use chrono::Local;

use crate::REVISIONS;

pub fn log_access(addr: SocketAddr, header: &UserAgent, route: &str) {
    const REQUIRED_USER_AGENT: &str = "KingsIsle Patcher";
    let eu_time = Local::now().format("%T (%d.%m.%Y)");

    if header.to_string().eq(REQUIRED_USER_AGENT) {
        log::info!("{addr} connected to {route} @ {eu_time}");
    } else {
        log::info!("[UNAUTHORIZED] {addr} connected to {route} @ {eu_time}");
    }
}

pub async fn explore_revisions() -> std::io::Result<()> {
    let mut dir = tokio::fs::read_dir(std::env::current_dir().unwrap().join("files")).await?;

    let mut revisions_vec: Vec<String> = Vec::new();
    while let Some(entry) = &dir.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            revisions_vec.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    let mut revisions = REVISIONS.write().unwrap();
    *revisions = revisions_vec;

    Ok(())
}
