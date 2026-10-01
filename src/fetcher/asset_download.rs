//! Download immutable, revision-qualified assets without discarding interrupted transfers.

use indicatif::ProgressBar;
use miette::{IntoDiagnostic, bail};
use reqwest::{Client, StatusCode, header};
use std::path::{Path, PathBuf};
use tokio::{fs, io::AsyncWriteExt};

fn partial_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".part");
    PathBuf::from(name)
}

/// Publish only a complete file of the manifest's expected size. A failed transfer
/// retains its `.part` file for the next pass; a server ignoring Range restarts it.
pub async fn download(
    client: &Client,
    url: &str,
    path: &Path,
    expected_size: u64,
    progress: &ProgressBar,
) -> miette::Result<()> {
    if fs::metadata(path)
        .await
        .is_ok_and(|m| m.len() == expected_size)
    {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.into_diagnostic()?;
    }
    let partial = partial_path(path);
    let offset = fs::metadata(&partial).await.map_or(0, |m| m.len());
    let offset = if offset > expected_size { 0 } else { offset };
    if offset != expected_size || !partial.exists() {
        transfer(client, url, &partial, offset, expected_size, progress).await?;
    }
    fs::rename(&partial, path).await.into_diagnostic()?;
    Ok(())
}

async fn transfer(
    client: &Client,
    url: &str,
    partial: &Path,
    offset: u64,
    expected_size: u64,
    progress: &ProgressBar,
) -> miette::Result<()> {
    // Range offsets describe raw asset bytes, never HTTP content-decoded bytes.
    let mut request = client.get(url).header(header::ACCEPT_ENCODING, "identity");
    if offset > 0 {
        request = request.header(header::RANGE, format!("bytes={offset}-"));
    }
    let mut response = request.send().await.into_diagnostic()?;
    let start = match response.status() {
        StatusCode::OK => 0,
        StatusCode::PARTIAL_CONTENT => {
            if expected_size == 0 {
                bail!("Unexpected partial response for empty asset: {url}");
            }
            let expected_range = format!("bytes {offset}-{}/{expected_size}", expected_size - 1);
            if response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|h| h.to_str().ok())
                != Some(expected_range.as_str())
            {
                bail!("Unexpected Content-Range for {url}");
            }
            offset
        }
        status => bail!("HTTP {status} downloading {url}"),
    };
    if response
        .content_length()
        .is_some_and(|length| length != expected_size - start)
    {
        bail!("Unexpected Content-Length for {url}");
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(start > 0)
        .truncate(start == 0)
        .open(partial)
        .await
        .into_diagnostic()?;
    progress.set_length(expected_size);
    progress.set_position(start);
    let mut received = start;
    while let Some(chunk) = response.chunk().await.into_diagnostic()? {
        received += chunk.len() as u64;
        if received > expected_size {
            bail!("Asset exceeds manifest size: {url}");
        }
        file.write_all(&chunk).await.into_diagnostic()?;
        progress.inc(chunk.len() as u64);
    }
    file.flush().await.into_diagnostic()?;
    if received != expected_size {
        bail!("Incomplete asset: {url} ({received}/{expected_size} bytes)");
    }
    Ok(())
}
