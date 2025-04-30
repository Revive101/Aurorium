use super::Revision;
use crate::errors::AssetFetcherError;
use reqwest::Client;
use roxmltree::{Document, Node};
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{File, create_dir_all, write},
    io::AsyncWriteExt,
};

/// Represents a single WAD file with its metadata
#[derive(Debug, Clone, Default)]
pub struct Asset {
    pub filename: String,
    pub size: i64,
    pub header_size: i64,
    pub compressed_header_size: i64,
    pub crc: i64,
    pub header_crc: i64,
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
    pub fn new(revision: Revision, concurrent_downloads: NonZeroUsize) -> Self {
        let save_directory = Path::new("files").join(&revision.revision);

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

        self.fetch_and_parse_xml(&xml_url, &xml_path).await?;

        Ok(())
    }

    // This function starts `n` parallel tasks to fetch multiple files
    async fn fetch_files() {}

    //
    async fn fetch_and_parse_xml(&mut self, url: &str, save_path: &PathBuf) -> Result<(), AssetFetcherError> {
        let response = Self::request_file(url).await?;
        let xml_text = response.text().await.unwrap_or_default();

        if !save_path.exists() {
            let sanitized_content = self.sanitize_content(&xml_text).await?;

            Self::write_to_file(&save_path, sanitized_content.as_bytes()).await?;
        }

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

    async fn download_and_save(&self, url: &str, path: &PathBuf) -> std::io::Result<()> {
        if path.exists() {
            // log::info!("File already exists, skipping download: {:?}", path);
            return Ok(());
        }

        let response = Self::request_file(url).await.unwrap();
        Self::write_to_file(path, &response.bytes().await.unwrap()).await
    }

    async fn request_file(url: &str) -> Result<reqwest::Response, reqwest::Error> {
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
