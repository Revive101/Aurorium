#![allow(unused)]

use super::structs::{Element, LatestFiles, RecordUnion};
use crate::revision_checker::revision::Revision;
use futures::{StreamExt, stream};
use reqwest::Client;
use std::{collections::VecDeque, io, path::PathBuf, process::exit};
use tokio::{
    fs::{File, create_dir_all, write},
    io::AsyncWriteExt,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Asset {
    pub filename: String,
    pub size: i64,
    pub header_size: i64,
    pub compressed_header_size: i64,
    pub crc: i64,
    pub header_crc: i64,
    pub already_fetched: bool,
}

#[derive(Debug, Clone, Default)]
struct AssetList {
    pub wad_list: VecDeque<Asset>,
    pub util_list: VecDeque<Asset>,
}

#[derive(Debug, Clone)]
pub struct AssetFetcher {
    pub list_file_url: String,
    pub url_prefix: String,
    pub revision: String,
    assets: AssetList,
    save_path: PathBuf,
    concurrent_downloads: usize,
}

impl AssetFetcher {
    pub fn new(revision: Revision, concurrent_downloads: usize) -> Self {
        Self {
            assets: AssetList::default(),
            revision: revision.clone().revision,
            url_prefix: revision.url_prefix,
            list_file_url: revision.list_file_url,
            save_path: PathBuf::from(format!("files/{}/", revision.revision)),
            concurrent_downloads,
        }
    }

    pub async fn load_index(&mut self) {
        let save_path = PathBuf::from("files").join(&self.revision);

        // Fetches the binary version of the LatestFileList
        if let Err(e) = self
            .download_and_save(&self.list_file_url, &save_path.join("LatestFileList.bin"))
            .await
        {
            log::error!("Error fetching LatestFileList.bin: {}", e);
            return;
        }

        // Fetches the XML version of the LatestFileList, which will be used to gather all assets
        let xml_url = self.list_file_url.replace("LatestFileList.bin", "LatestFileList.xml");
        match self.download_and_parse_xml(&xml_url, &save_path.join("LatestFileList.xml")).await {
            Ok(_) => {
                self.fetch_files(&self.assets.wad_list, &save_path).await;
                self.fetch_files(&self.assets.util_list, &save_path).await;
            }
            Err(why) => log::error!("Error processing LatestFileList.xml: {why}"),
        }
    }

    async fn fetch_files(&self, file_list: &VecDeque<Asset>, base_path: &PathBuf) {
        stream::iter(file_list.iter().cloned().map(|file| {
            let url = format!("{}/{}", self.url_prefix, file.filename);
            let path = base_path.join(&file.filename);

            async move {
                if !path.exists() {
                    match Self::request_file(&url).await {
                        Ok(res) => match Self::write_to_file_chunked(&path, res).await {
                            Err(e) => {
                                log::warn!("[❌] Could not fetch {}: {}", file.filename, e);
                            }
                            _ => {
                                log::info!("[✔] Fetched {}", file.filename);
                            }
                        },
                        Err(e) => log::warn!("[❌] Could not fetch {}: {}", file.filename, e),
                    };
                } else {
                    log::info!("Skipping {:?}", &file.filename);
                }
            }
        }))
        .buffer_unordered(self.concurrent_downloads)
        .collect::<Vec<_>>()
        .await;
    }

    async fn download_and_parse_xml(&mut self, url: &str, save_path: &PathBuf) -> io::Result<()> {
        let response = Self::request_file(url).await.unwrap();
        let xml_text = response.text().await.unwrap_or_default();

        if !save_path.exists() {
            Self::write_to_file(&save_path, xml_text.as_bytes()).await?;
        }

        self.parse_and_store_elements(xml_text)?;
        Ok(())
    }

    fn parse_and_store_elements(&mut self, xml_text: String) -> io::Result<()> {
        let config = quickxml_to_serde::Config::new_with_defaults();
        let json = quickxml_to_serde::xml_string_to_json(xml_text, &config).unwrap().to_string();
        let parsed: LatestFiles = serde_json::from_str(&json).unwrap();

        for (_, v) in parsed.latest_file_list {
            match v.record {
                RecordUnion::PurpleRecord(purple_record) => self.add_file_to_list(&purple_record),
                RecordUnion::RecordElementArray(records) => {
                    for r in records {
                        self.add_file_to_list(&r);
                    }
                }
            }
        }

        Ok(())
    }

    async fn download_and_save(&self, url: &str, path: &PathBuf) -> io::Result<()> {
        if path.exists() {
            log::info!("File already exists, skipping download: {:?}", path);
            return Ok(());
        }

        let response = Self::request_file(url).await.unwrap();
        Self::write_to_file(path, &response.bytes().await.unwrap()).await
    }

    fn add_file_to_list<T: Element>(&mut self, record: &T) {
        if let Some(src_file_name) = record.get_filename() {
            let file = Asset {
                filename: src_file_name,
                size: record.get_size(),
                header_size: record.get_header_size(),
                compressed_header_size: record.get_compressed_header_size(),
                crc: record.get_crc(),
                header_crc: record.get_header_crc(),
                already_fetched: false,
            };

            if file.filename.ends_with(".wad") {
                self.assets.wad_list.push_back(file);
            } else {
                self.assets.util_list.push_back(file);
            }
        }
    }

    async fn request_file(url: &str) -> Result<reqwest::Response, reqwest::Error> {
        let client = Client::new();
        client.get(url).header("User-Agent", "KingsIsle Patcher").send().await
    }

    async fn write_to_file(path: &PathBuf, content: &[u8]) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }
        write(path, content).await
    }

    async fn write_to_file_chunked(path: &PathBuf, mut response: reqwest::Response) -> io::Result<()> {
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
