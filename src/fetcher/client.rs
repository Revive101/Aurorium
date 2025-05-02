use crate::{
    errors::AssetFetcherError,
    models::asset::{Asset, AssetList},
    revision::{Revision, xml_parser::parse_xml},
};
use futures_util::{
    StreamExt,
    stream::{self},
};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use roxmltree::{Document, Node};
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
    pub fn new(revision: Revision, concurrent_downloads: NonZeroUsize, save_directory_name: PathBuf) -> Self {
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
        let response = self.client.get(url).send().await?;
        let xml_text = response.text().await.unwrap_or_default();

        let sanitized_content = self.sanitize_content(&xml_text).await?;
        self.propagate_file_list(&sanitized_content)?;

        if !save_path.exists() {
            Self::write_to_file(&save_path, &sanitized_content.into_bytes()).await?;
        }

        Ok(())
    }

    fn propagate_file_list(&mut self, xml_text: &str) -> Result<(), AssetFetcherError> {
        let records = parse_xml(xml_text)?;
        let (wads, utils) = records.into_iter().partition(|f| f.filename.ends_with(".wad"));

        self.assets.wads = wads;
        self.assets.utils = utils;

        Ok(())
    }

    // ts looks so ugly like fr 🥀🥀
    ///////////////////////////////////////
    async fn sanitize_content(&mut self, text: &str) -> Result<String, AssetFetcherError> {
        let doc = Document::parse(text)?;
        let root = doc.root_element();

        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" ?>\n<LatestFileList>\n");

        for child in root.children() {
            if child.is_element() && !matches!(child.tag_name().name(), "_TableList" | "About") {
                output.push_str(&Self::node_to_string(child));
                output.push('\n');
            }
        }

        output.push_str("</LatestFileList>");
        Ok(output)
    }

    fn node_to_string(node: Node) -> String {
        let mut s = String::new();
        s.push_str(&format!("<{}>", node.tag_name().name()));

        for child in node.children() {
            match () {
                _ if child.is_element() => s.push_str(&Self::node_to_string(child)),
                _ if child.is_text() => s.push_str(child.text().unwrap_or("")),
                _ => (),
            }
        }

        s.push_str(&format!("</{}>", node.tag_name().name()));
        s
    }
    //////////////////////////////////////

    // This function starts `n` parallel tasks to fetch multiple files
    async fn fetch_files(&self, file_list: &Vec<Asset>, base_path: &PathBuf) {
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
                                Err(e) => println!("[❌] Could not fetch {}: {e}", file.filename),
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
