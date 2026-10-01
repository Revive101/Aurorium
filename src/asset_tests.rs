//! Regression coverage for interrupted assets and unindexed WAD header sidecars.

use crate::{
    AppState,
    config::AppConfig,
    db::Database,
    fetcher::asset_download,
    revision::{Asset, Revision},
};
use axum::{
    body::{Body, to_bytes},
    extract::{Path, State},
    http::Request,
    response::IntoResponse,
};
use indicatif::ProgressBar;
use reqwest::Client;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "aurorium-assets-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A deliberately small HTTP fixture can disconnect mid-body, unlike a normal
/// framework response which would repair Content-Length for us.
async fn server(response: &'static str) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/asset", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(socket.read_u8().await.unwrap());
        }
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
        String::from_utf8(request).unwrap().to_lowercase()
    });
    (url, handle)
}

#[tokio::test]
async fn interrupted_body_is_retained_and_resumed() {
    let scratch = Scratch::new();
    let path = scratch.0.join("asset");
    let client = Client::new();
    let progress = ProgressBar::hidden();
    let (url, request) =
        server("HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabc").await;
    assert!(
        asset_download::download(&client, &url, &path, 6, &progress)
            .await
            .is_err()
    );
    request.await.unwrap();
    assert!(!path.exists());
    assert_eq!(std::fs::read(scratch.0.join("asset.part")).unwrap(), b"abc");
    let (url, request) = server("HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nContent-Range: bytes 3-5/6\r\nConnection: close\r\n\r\ndef").await;
    asset_download::download(&client, &url, &path, 6, &progress)
        .await
        .unwrap();
    assert!(request.await.unwrap().contains("range: bytes=3-"));
    assert_eq!(std::fs::read(path).unwrap(), b"abcdef");
    assert!(!scratch.0.join("asset.part").exists());
}

#[tokio::test]
async fn ignored_range_restarts_without_appending_duplicate_bytes() {
    let scratch = Scratch::new();
    let path = scratch.0.join("asset");
    std::fs::write(scratch.0.join("asset.part"), b"abc").unwrap();
    let (url, request) =
        server("HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabcdef").await;
    asset_download::download(&Client::new(), &url, &path, 6, &ProgressBar::hidden())
        .await
        .unwrap();
    request.await.unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"abcdef");
}

#[tokio::test]
async fn wrong_range_does_not_corrupt_partial_file() {
    let scratch = Scratch::new();
    let path = scratch.0.join("asset");
    std::fs::write(scratch.0.join("asset.part"), b"abc").unwrap();
    let (url, request) = server("HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nContent-Range: bytes 1-3/6\r\nConnection: close\r\n\r\nbcd").await;
    assert!(
        asset_download::download(&Client::new(), &url, &path, 6, &ProgressBar::hidden())
            .await
            .is_err()
    );
    request.await.unwrap();
    assert!(!path.exists());
    assert_eq!(std::fs::read(scratch.0.join("asset.part")).unwrap(), b"abc");
}

#[tokio::test]
async fn errors_are_not_cached_and_wrong_size_files_are_replaced() {
    let scratch = Scratch::new();
    let path = scratch.0.join("asset");
    std::fs::write(&path, b"bad").unwrap();
    let (url, request) =
        server("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
    assert!(
        asset_download::download(&Client::new(), &url, &path, 6, &ProgressBar::hidden())
            .await
            .is_err()
    );
    request.await.unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"bad");
    let (url, request) =
        server("HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabcdef").await;
    asset_download::download(&Client::new(), &url, &path, 6, &ProgressBar::hidden())
        .await
        .unwrap();
    request.await.unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"abcdef");
}

#[tokio::test]
async fn cached_wad_still_downloads_its_missing_header() {
    let scratch = Scratch::new();
    std::fs::create_dir_all(scratch.0.join("rev/Data")).unwrap();
    std::fs::write(scratch.0.join("rev/Data/test.wad"), b"existing").unwrap();
    let (url, request) =
        server("HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nheader").await;
    let patcher = crate::wizard_patcher::WizardPatcher {
        list_file_url: String::new(),
        url_prefix: url,
        revision: Revision {
            name: "rev".into(),
            number: 1,
        },
    };
    let concurrency = std::num::NonZeroUsize::new(1).unwrap();
    let fetcher = crate::fetcher::asset_fetcher::AssetFetcher::new(
        patcher,
        &concurrency,
        &scratch.0,
        vec![Asset {
            file_name: "Data/test.wad".into(),
            size: 8,
            compressed_header_size: 6,
            ..Asset::default()
        }],
    )
    .unwrap();
    fetcher.fetch_assets().await.unwrap();
    assert!(request.await.unwrap().contains("/data/test.wad.hdr.gz"));
    assert_eq!(
        std::fs::read(scratch.0.join("rev/Data/test.wad.hdr.gz")).unwrap(),
        b"header"
    );
}

#[tokio::test]
async fn wad_header_uses_owning_revision_and_supports_ranges() {
    let scratch = Scratch::new();
    let db = Database::init(scratch.0.join("test.db").to_str().unwrap())
        .await
        .unwrap();
    let asset = Asset {
        file_name: "Data/test.wad".into(),
        size: 8,
        crc: 42,
        compressed_header_size: 6,
        ..Asset::default()
    };
    for (name, number) in [("old", 1), ("new", 2)] {
        db.insert_new_revision(
            Revision {
                name: name.into(),
                number,
            },
            vec![asset.clone()],
        )
        .await
        .unwrap();
    }
    std::fs::create_dir_all(scratch.0.join("old/Data")).unwrap();
    std::fs::write(scratch.0.join("old/Data/test.wad.hdr.gz"), b"abcdef").unwrap();
    let mut config = AppConfig::default();
    config.fetcher.save_directory = scratch.0.to_str().unwrap().into();
    let request = Request::builder()
        .header("Range", "bytes=1-3")
        .body(Body::empty())
        .unwrap();
    let response = crate::routes::file::file(
        State(AppState::new(config, db)),
        Path(("new".into(), "Data/test.wad.hdr.gz".into())),
        crate::utils::ConnectionAddr("127.0.0.1".into()),
        request,
    )
    .await
    .unwrap()
    .into_response();
    assert_eq!(response.status(), 206);
    assert_eq!(response.headers()["content-range"], "bytes 1-3/6");
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        b"bcd"
    );
}
