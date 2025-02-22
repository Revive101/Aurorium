use crate::{REVISIONS, fetcher::fetcher::AssetFetcher};

use super::revision::Revision;
use std::env::current_dir;
use tokio::fs::{create_dir_all, read_dir};

pub struct RevisionChecker {
    concurrent_downloads: usize,
}

impl RevisionChecker {
    pub async fn new(concurrent_downloads: usize) -> std::io::Result<Self> {
        let mut revisions = REVISIONS.write().await;
        let path = current_dir().unwrap().join("files");

        if !path.exists() {
            create_dir_all(path.clone()).await.unwrap();
        }

        let mut path_content = read_dir(path).await?;
        while let Some(entry) = &path_content.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                revisions.push(entry.file_name().to_string_lossy().to_string());
            }
        }

        Ok(Self { concurrent_downloads })
    }

    pub async fn check_latest_revision_fetched(&mut self) {
        let revision = Revision::check_for_new_revision().await.unwrap();

        {
            let revisions = REVISIONS.read().await;
            if revisions.contains(&revision.revision) {
                log::info!("Newest revision is already fetched!");
                return;
            }
        }

        let mut request = AssetFetcher::new(revision.clone(), self.concurrent_downloads);
        request.load_index().await;

        let mut revisions = REVISIONS.write().await;
        revisions.push(revision.revision);
    }
}
