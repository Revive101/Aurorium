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
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{File, create_dir_all, write},
    io::AsyncWriteExt,
};

/// Handles fetching and managing game assets
pub struct AssetFetcher {
    pub list_file_url: String,
    pub url_prefix: String,
    pub revision: String,
    pub assets: AssetList,
    save_directory: PathBuf,
    concurrent_downloads: NonZeroUsize,
    client: Client,
}

impl AssetFetcher {
    pub fn new<P>(patch_info: &PatchInfo, concurrent_downloads: NonZeroUsize, save_directory_name: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            assets: AssetList::default(),
            revision: patch_info.revision.clone(),
            url_prefix: patch_info.url_prefix.clone(),
            list_file_url: patch_info.list_file_url.clone(),
            save_directory: save_directory_name.as_ref().join(&patch_info.revision),
            concurrent_downloads,
            client: Client::builder().user_agent("KingsIsle Patcher").build().unwrap(),
        }
    }

    pub async fn fetch_index(&mut self) -> Result<&mut Self, AssetFetcherError> {
        println!("Fetching LatestFileList...");
        // Fetches the BIN version of `LatestFileList`
        let bin_path = self.save_directory.join("LatestFileList.bin");
        if !bin_path.exists() {
            let response = self.client.get(&self.list_file_url).send().await?;
            Self::write_to_file_chunked(&bin_path, response).await?;
        }

        // Fetches the XML version of `LatestFileList`
        let xml_url = self.list_file_url.replace(".bin", ".xml");
        let xml_path = self.save_directory.join("LatestFileList.xml");

        self.process_xml(&xml_url, &xml_path).await?;

        Ok(self)
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
    pub async fn fetch_files(&self, file_list: Vec<Asset>) {
        println!("Fetching files...");

        let multi_pb = MultiProgress::new();

        let main_style = ProgressStyle::with_template("{spinner:.blue} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("#>-");

        let main_pb = multi_pb.add(ProgressBar::new(file_list.len() as u64));
        main_pb.set_style(main_style);

        let download_style = ProgressStyle::with_template(
            "{msg:.cyan} {spinner:.blue} [{elapsed_precise}] [{wide_bar:.green/blue}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-");

        let download_futures = file_list.into_iter().map(|file| {
            let client = self.client.clone();
            let url_prefix = self.url_prefix.clone();
            let save_dir = self.save_directory.clone();
            // Progress bar
            let multi_pb = multi_pb.clone();
            let main_pb = main_pb.clone();
            let style = download_style.clone();

            async move {
                let url = format!("{}/{}", url_prefix, file.filename);
                let path = save_dir.join(&file.filename);

                if !path.exists() {
                    let file_pb = multi_pb.add(ProgressBar::new_spinner());
                    file_pb.set_style(style);
                    file_pb.set_message(format!("Downloading {}", file.filename));

                    if let Ok(res) = client.get(&url).send().await {
                        file_pb.set_length(res.content_length().unwrap_or(0));

                        if let Err(e) = Self::write_to_file_chunked_with_progress(&path, res, &file_pb).await {
                            file_pb.finish_with_message(format!("Failed: {}", e));
                        } else {
                            file_pb.finish_with_message("Done");
                        }

                        multi_pb.remove(&file_pb);
                    }
                }

                main_pb.inc(1);
            }
        });

        stream::iter(download_futures)
            .buffer_unordered(self.concurrent_downloads.get())
            .collect::<Vec<()>>()
            .await;

        main_pb.finish_with_message("All downloads complete.");
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

    async fn write_to_file_chunked_with_progress(
        path: &std::path::Path,
        mut response: reqwest::Response,
        pb: &ProgressBar,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }

        let mut file = File::create(path).await?;
        while let Some(chunk) = response.chunk().await.unwrap() {
            file.write_all(&chunk).await?;
            pb.inc(chunk.len() as u64);
        }

        Ok(())
    }
}
