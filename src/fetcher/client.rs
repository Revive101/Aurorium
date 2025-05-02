use crate::{
    errors::AssetFetcherError,
    models::asset::{Asset, AssetList},
    patch_info::PatchInfo,
    xml_parser::{parse_xml, sanitize_content},
};
use futures_util::{
    StreamExt,
    stream::{self},
};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::{num::NonZeroUsize, path::PathBuf};
use tokio::{
    fs::{File, create_dir_all, write},
    io::AsyncWriteExt,
};

/// Handles fetching and managing game assets
pub struct AssetFetcher {
    pub list_file_url: String,
    pub url_prefix: String,
    pub revision: String,
    assets: AssetList,
    save_directory: PathBuf,
    concurrent_downloads: NonZeroUsize,
    client: Client,
}

impl AssetFetcher {
    pub fn new(revision: PatchInfo, concurrent_downloads: NonZeroUsize, save_directory_name: PathBuf) -> Self {
        Self {
            assets: AssetList::default(),
            revision: revision.revision.clone(),
            url_prefix: revision.url_prefix,
            list_file_url: revision.list_file_url,
            save_directory: save_directory_name.join(&revision.revision),
            concurrent_downloads,
            client: Client::builder().user_agent("KingsIsle Patcher").build().unwrap(),
        }
    }

    pub async fn fetch_index(&mut self) -> Result<(), AssetFetcherError> {
        println!("Fetching LatestFileList...");
        // Fetches the BIN version of `LatestFileList`
        let bin_path = self.save_directory.join("LatestFileList.bin");
        let response = self.client.get(&self.list_file_url).send().await?;
        Self::write_to_file_chunked(&bin_path, response).await?;

        // Fetches the XML version of `LatestFileList`
        let xml_url = self.list_file_url.replace(".bin", ".xml");
        let xml_path = self.save_directory.join("LatestFileList.xml");

        if let Ok(_) = self.process_xml(&xml_url, &xml_path).await {
            self.fetch_files(&self.assets.wads, &self.save_directory).await;
            self.fetch_files(&self.assets.utils, &self.save_directory).await;
        }

        Ok(())
    }

    async fn process_xml(&mut self, url: &str, save_path: &PathBuf) -> Result<(), AssetFetcherError> {
        println!("Processing LatestFileList.xml...");

        let response = self.client.get(url).send().await?;
        let xml_text = response.text().await.unwrap_or_default();
        let sanitized_content = sanitize_content(&xml_text).await?;

        let (wads, utils) = parse_xml(&sanitized_content)?;
        self.assets.wads = wads;
        self.assets.utils = utils;

        if !save_path.exists() {
            Self::write_to_file(&save_path, &sanitized_content.into_bytes()).await?;
        }

        Ok(())
    }

    // This function starts `n` parallel tasks to fetch multiple files
    async fn fetch_files(&self, file_list: &Vec<Asset>, base_path: &PathBuf) {
        println!("Fetching files...");

        let style = ProgressStyle::with_template("{spinner:.blue} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("#>-");

        let pb = ProgressBar::new(file_list.len() as u64);
        pb.set_style(style);

        stream::iter(file_list.iter().map(|file| {
            let url = format!("{}/{}", self.url_prefix, file.filename);
            let path = base_path.join(&file.filename);

            {
                let progress_bar = pb.clone();
                async move {
                    if !path.exists() {
                        match self.client.get(&url).send().await {
                            Ok(res) => match Self::write_to_file_chunked(&path, res).await {
                                Err(e) => println!("[❌] Could not write file {}: {e}", file.filename),
                                _ => {}
                            },
                            Err(e) => println!("[❌] Could not fetch {}: {}", file.filename, e),
                        }
                    }

                    progress_bar.inc(1);
                }
            }
        }))
        .buffer_unordered(self.concurrent_downloads.get())
        .collect::<Vec<_>>()
        .await;

        pb.finish();
        println!("Done!");
    }

    async fn write_to_file(path: &PathBuf, content: &[u8]) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }

        write(path, content).await
    }

    async fn write_to_file_chunked(path: &PathBuf, mut response: reqwest::Response) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }

        let mut file = File::create(path).await?;
        while let Some(chunk) = response.chunk().await.unwrap() {
            file.write_all(&chunk).await?;
        }

        Ok(())
    }
}
