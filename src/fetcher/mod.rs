use crate::errors::FetcherTraitError;
use indicatif::ProgressBar;
use reqwest::{Client, Response};
use std::path::{Path, PathBuf};
use tokio::{
    fs::create_dir_all,
    io::{AsyncWriteExt, BufWriter},
};

pub mod asset_fetcher;
pub mod manifest_fetcher;

pub trait Fetcher {
    async fn fetch(client: &Client, url: &str) -> miette::Result<Response> {
        Ok(client
            .get(url)
            .send()
            .await
            .and_then(Response::error_for_status)
            .map_err(|e| FetcherTraitError::Fetch(e, url.to_string()))?)
    }

    /// Streams an HTTP response to disk, optionally driving a progress bar.
    ///
    /// # Errors
    /// Returns an error if any of the following conditions are met:
    /// - Parent directories cannot be created.
    /// - The temporary `.part` file cannot be created or written to.
    /// - A chunk from the HTTP response body cannot be read.
    /// - The `.part` file cannot be renamed to its final destination path.
    ///
    /// # TODO
    /// Implement resuming downloads by checking for a .part file and continuing from where it left off. (Their server supports range requests, so this should be possible.)
    async fn write_to_file_streamed<P>(
        path: P,
        mut response: reqwest::Response,
        progress: Option<&ProgressBar>,
    ) -> miette::Result<()>
    where
        P: AsRef<Path>,
    {
        let final_path = path.as_ref();
        let part_path = Self::part_path(final_path);

        let file = match tokio::fs::File::create(&part_path).await {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Some(parent) = final_path.parent() {
                    create_dir_all(parent)
                        .await
                        .map_err(FetcherTraitError::CreateDir)?;
                }
                tokio::fs::File::create(&part_path)
                    .await
                    .map_err(FetcherTraitError::Io)?
            }
            Err(e) => return Err(FetcherTraitError::Io(e).into()),
        };
        let mut writer = BufWriter::with_capacity(128 * 1024, file); // TODO: Let the user configure this buffer size(?)

        // Stream response to file in chunks (to avoid loading the entire file into memory)
        let result: miette::Result<()> = async {
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|e| miette::miette!("Failed to read chunk: {e}"))?
            {
                writer
                    .write_all(&chunk)
                    .await
                    .map_err(FetcherTraitError::Io)?;

                if let Some(pb) = progress {
                    pb.inc(chunk.len() as u64);
                }
            }

            writer.flush().await.map_err(FetcherTraitError::Io)?;
            Ok(())
        }
        .await;

        // If there was an error during the download, remove the partial file and return the error
        if let Err(e) = result {
            let _ = tokio::fs::remove_file(&part_path).await;
            return Err(e);
        }

        // Rename the .part file to the final filename
        tokio::fs::rename(&part_path, final_path)
            .await
            .map_err(FetcherTraitError::Rename)?;

        Ok(())
    }

    fn part_path(path: &Path) -> PathBuf {
        let mut part_os = path.as_os_str().to_owned();
        part_os.push(".part");
        PathBuf::from(part_os)
    }
}
