use super::{Revision, xml_parser::parse_xml};
use crate::errors::AssetFetcherError;
use futures_util::{
    StreamExt,
    stream::{self},
};
use reqwest::Client;
use roxmltree::{Document, Node};
use std::{num::NonZeroUsize, path::PathBuf};
use tokio::{
    fs::{File, create_dir_all, write},
    io::AsyncWriteExt,
};

/// Represents a single WAD file with its metadata
#[derive(Debug, Default)]
pub struct Asset {
    pub filename: String,
    pub file_type: String,
    pub size: u64,
    pub header_size: u64,
    pub compressed_header_size: u64,
    pub crc: u64,
    pub header_crc: u64,
}

/// Categorizes different types of game assets
#[derive(Debug, Default)]
struct AssetList {
    pub wads: Vec<Asset>,
    pub utils: Vec<Asset>,
}

/// Handles fetching and managing game assets from a remote server
pub struct AssetFetcher {
    pub list_file_url: String,
    pub url_prefix: String,
    pub revision: String,
    assets: AssetList,
    save_directory: PathBuf,
    concurrent_downloads: NonZeroUsize,
}

impl AssetFetcher {
    pub fn new(revision: Revision, concurrent_downloads: NonZeroUsize, save_directory_name: PathBuf) -> Self {
        let save_directory = save_directory_name.join(&revision.revision);

        Self {
            assets: AssetList::default(),
            revision: revision.revision.clone(),
            url_prefix: revision.url_prefix,
            list_file_url: revision.list_file_url,
            save_directory,
            concurrent_downloads,
        }
    }

    pub async fn fetch_index(&mut self) -> Result<(), AssetFetcherError> {
        // Fetches the BIN version of `LatestFileList`
        let list_file_path = self.save_directory.join("LatestFileList.bin");
        self.download_and_save(&self.list_file_url, &list_file_path).await?;

        // Fetches the XML version of `LatestFileList`
        let xml_url = self.list_file_url.replace("LatestFileList.bin", "LatestFileList.xml");
        let xml_path = self.save_directory.join("LatestFileList.xml");

        match self.fetch_and_parse_xml(&xml_url, &xml_path).await {
            Ok(_) => {
                self.fetch_files(&self.assets.wads, &self.save_directory).await;
                self.fetch_files(&self.assets.utils, &self.save_directory).await;
            }
            Err(e) => {
                println!("Failed to parse XML: {e}");
            }
        }

        Ok(())
    }

    //
    async fn fetch_and_parse_xml(&mut self, url: &str, save_path: &PathBuf) -> Result<(), AssetFetcherError> {
        let response = Self::send_request(url).await?;
        let xml_text = response.text().await.unwrap_or_default();

        let sanitized_content = self.sanitize_content(&xml_text).await?;
        self.propogate_file_list(&sanitized_content);

        if !save_path.exists() {
            Self::write_to_file(&save_path, &sanitized_content.into_bytes()).await?;
        }

        Ok(())
    }

    fn propogate_file_list(&mut self, xml_text: &str) {
        let records = parse_xml(xml_text).unwrap();

        let mut wads = Vec::new();
        let mut utils = Vec::new();

        for file in records {
            if file.filename.ends_with(".wad") {
                wads.push(file);
            } else {
                utils.push(file);
            }
        }

        self.assets.wads = std::mem::take(&mut wads);
        self.assets.utils = std::mem::take(&mut utils);
    }

    // ts looks so ugly like fr 🥀🥀
    ///////////////////////////////////////
    async fn sanitize_content(&mut self, text: &str) -> Result<String, AssetFetcherError> {
        let doc = Document::parse(text)?;
        let root = doc.root_element();

        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" ?>\n<LatestFileList>\n");

        for child in root.children() {
            match child.tag_name().name() {
                "_TableList" | "About" => continue,
                _ => {
                    if child.is_element() {
                        output.push_str(&Self::node_to_string(child));
                        output.push('\n');
                    }
                }
            }
        }

        output.push_str("</LatestFileList>");
        Ok(output)
    }

    fn node_to_string(node: Node) -> String {
        let mut s = String::new();
        s.push_str(&format!("<{}>", node.tag_name().name()));

        for child in node.children() {
            if child.is_element() {
                s.push_str(&Self::node_to_string(child));
            } else if child.is_text() {
                s.push_str(child.text().unwrap_or(""));
            }
        }

        s.push_str(&format!("</{}>", node.tag_name().name()));
        s
    }
    //////////////////////////////////////

    // This function starts `n` parallel tasks to fetch multiple files
    async fn fetch_files(&self, file_list: &Vec<Asset>, base_path: &PathBuf) {
        stream::iter(file_list.iter().map(|file| {
            let url = format!("{}/{}", self.url_prefix, file.filename);
            let path = base_path.join(&file.filename);

            async move {
                if !path.exists() {
                    match Self::send_request(&url).await {
                        Ok(res) => match Self::write_to_file_chunked(&path, res).await {
                            Err(e) => println!("[❌] Could not fetch {}: {e}", file.filename),
                            _ => {
                                println!("[✔] Fetched {}", file.filename);
                            }
                        },
                        Err(e) => println!("[❌] Could not fetch {}: {}", file.filename, e),
                    }
                } else {
                    println!("Skipping {:?}", &file.filename)
                }
            }
        }))
        .buffer_unordered(self.concurrent_downloads.get())
        .collect::<Vec<_>>()
        .await;
    }

    async fn download_and_save(&self, url: &str, path: &PathBuf) -> std::io::Result<()> {
        if path.exists() {
            // log::info!("File already exists, skipping download: {:?}", path);
            return Ok(());
        }

        let response = Self::send_request(url).await.unwrap();
        Self::write_to_file(path, &response.bytes().await.unwrap()).await
    }

    async fn send_request(url: &str) -> Result<reqwest::Response, reqwest::Error> {
        let client = Client::new();
        client.get(url).header("User-Agent", "KingsIsle Patcher").send().await
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
