/*
    Copyright (c) 2023 Phill030. All rights reserved.
    This code is exclusive to Revive101.

    Unauthorized use, reproduction, or distribution of this code,
    in whole or in part, by any party outside of Revive101 is prohibited.
*/

use console::{style, Emoji};
use futures::StreamExt;
use quickxml_to_serde::Config;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub filelist_url: String,
    pub file_url: String,
    pub files: FileList,
    max_concurrent_downloads: usize,
}

#[derive(Debug, Clone)]
pub struct FileList {
    pub revision: String,
    pub wad_list: Vec<File>,
    pub util_list: Vec<File>,
}

#[derive(Debug, Clone)]
pub struct File {
    pub filename: String,
    pub size: i64,
    pub header_size: i64,
    pub compressed_header_size: i64,
    pub crc: i64,
    pub header_crc: i64,
}

// http://versionec.us.wizard101.com/WizPatcher/V_r739602.Wizard_1_520_0_Live/Windows/LatestFileList.xml
// https://versionec.us.wizard101.com/WizPatcher/V_r739602.Wizard_1_520_0_Live/LatestBuild/Data/GameData/Khrysalis-Interiors-KR_Z09_I26_PaleBeautyBot02.wad
// ! https://app.quicktype.io/ to generate structs

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static TRUCK: Emoji<'_, '_> = Emoji("🚚  ", "");
static LINK: Emoji<'_, '_> = Emoji("🔗  ", "");
static BOX: Emoji<'_, '_> = Emoji("📦  ", "");

impl HttpRequest {
    pub async fn new(revision: String, concurrent: usize) -> HttpRequest {
        const BASE_URL: &str = "http://versionec.us.wizard101.com/WizPatcher";
        println!(
            "{} {}Resolving revision...",
            style("[1/6]").bold().dim(),
            LOOKING_GLASS
        );

        HttpRequest {
            filelist_url: format!("{BASE_URL}/{revision}/Windows"),
            file_url: format!("{BASE_URL}/{revision}/LatestBuild"),
            max_concurrent_downloads: concurrent,
            files: FileList {
                wad_list: Vec::new(),
                util_list: Vec::new(),
                revision: revision.replace("\r\n", ""),
            },
        }
    }

    pub async fn propogate_filelist(&mut self) {
        println!(
            "{} {}Fetching LatestFileList...",
            style("[2/6]").bold().dim(),
            TRUCK
        );

        let save_path = PathBuf::from("files").join(&self.files.revision);
        let bin_url = &format!("{}/LatestFileList.bin", &self.filelist_url);

        // LatestFileList.bin TODO: Move this into their own functions!!
        if let Ok(res) = request_file(&bin_url).await {
            if !save_path.join("utils").join("LatestFileList.bin").exists() {
                if let Err(_) = write_to_file(
                    &save_path.join("utils").join("LatestFileList.bin"),
                    &res.bytes().await.unwrap().to_vec(),
                )
                .await
                {
                    log::error!("Could not save LatestFileList.bin");
                }
            }
        } else {
            log::error!("Could not fetch LatestFileList.bin")
        };

        let xml_url = &format!("{}/LatestFileList.xml", &self.filelist_url);
        // LatestFileList.xml TODO: Move this into their own functions!!
        match request_file(&xml_url).await {
            Ok(res) => {
                let xml_text = res.text().await.unwrap_or(String::new());

                if !save_path.join("LatestFileList.xml").exists() {
                    if let Err(_) = write_to_file(
                        &save_path.join("LatestFileList.xml"),
                        &xml_text.as_bytes().to_vec(),
                    )
                    .await
                    {
                        log::error!("Could not save LatestFileList.xml");
                    }
                }

                let config = Config::new_with_defaults();
                let json = quickxml_to_serde::xml_string_to_json(xml_text, &config)
                    .unwrap()
                    .to_string();

                let parsed = serde_json::from_str::<LatestFiles>(json.as_str())
                    .expect("Could not parse JSON");

                parsed
                    .latest_file_list
                    .iter()
                    .for_each(|(_k, v)| match &v.record {
                        // What is this shit? 😩
                        RecordUnion::PurpleRecord(r) => {
                            if r.src_file_name.is_some() {
                                let filename =
                                    r.src_file_name.as_ref().unwrap().value.as_ref().unwrap();
                                let size = r.size.as_ref().unwrap().value;
                                let header_size = r.header_size.as_ref().unwrap().value;
                                let compressed_header_size =
                                    r.compressed_header_size.as_ref().unwrap().value;
                                let crc = r.crc.as_ref().unwrap().value;
                                let header_crc = r.header_crc.as_ref().unwrap().value;

                                self.files.wad_list.push(File {
                                    filename: filename.clone(),
                                    size,
                                    header_size,
                                    compressed_header_size,
                                    crc,
                                    header_crc,
                                });
                            }
                        }
                        // Bro give me some time to rest 😩😞
                        RecordUnion::RecordElementArray(r) => {
                            for el in r {
                                if el.src_file_name.is_some() {
                                    let filename =
                                        el.src_file_name.as_ref().unwrap().value.as_ref().unwrap();
                                    let size = el.size.as_ref().unwrap().value;
                                    let header_size = el.header_size.as_ref().unwrap().value;
                                    let compressed_header_size =
                                        el.compressed_header_size.as_ref().unwrap().value;
                                    let crc = el.crc.as_ref().unwrap().value;
                                    let header_crc = el.header_crc.as_ref().unwrap().value;

                                    if filename.ends_with(".wad") {
                                        self.files.wad_list.push(File {
                                            filename: filename.clone(),
                                            size,
                                            header_size,
                                            compressed_header_size,
                                            crc,
                                            header_crc,
                                        });
                                    } else {
                                        self.files.util_list.push(File {
                                            filename: filename.clone(),
                                            size,
                                            header_size,
                                            compressed_header_size,
                                            crc,
                                            header_crc,
                                        });
                                    }
                                }
                            }
                        }
                    });

                println!(
                    "{} {}Inserted {} wad files & {} util files...",
                    style("[3/6]").bold().dim(),
                    LINK,
                    &self.files.wad_list.len(),
                    &self.files.util_list.len()
                );

                Self::fetch_wads(self, save_path).await;
            }
            Err(_) => log::error!("Could not fetch LatestFileList.xml"),
        };
    }

    /// This is pure 🌟 Magic 🌟
    async fn fetch_wads(&mut self, save_path: PathBuf) {
        let url = self.file_url.clone();
        futures::stream::iter(self.files.wad_list.clone().into_iter().map(|wad| {
            let url_cloned = url.clone();

            let file_name = PathBuf::from(&wad.filename);
            let file = file_name.file_name().unwrap().to_string_lossy().to_string();
            let path = save_path.join("wads").join(&file);

            async move {
                if !path.exists() {
                    match request_file(format!("{}/{}", &url_cloned, &wad.filename)).await {
                        Ok(res) => {
                            let bytes = res
                                .bytes()
                                .await
                                .expect("Could not convert to bytes!")
                                .to_vec();

                            write_to_file(&path, &bytes).await.unwrap();

                            log::info!("[✔] Fetched {}", wad.filename);
                        }
                        Err(why) => {
                            log::warn!("[❌] Could not fetch {}, {}", wad.filename, why);
                        }
                    };
                } else {
                    log::info!("Skipping {:#?}", &file);
                }
            }
        }))
        .buffer_unordered(self.max_concurrent_downloads)
        .collect::<Vec<()>>()
        .await;

        println!(
            "{} {}fetched {} wad files",
            style("[4/6]").bold().dim(),
            BOX,
            &self.files.wad_list.len(),
        );

        Self::fetch_utils(self, save_path).await;
    }

    /// This is pure 🌟 Magic 🌟
    async fn fetch_utils(&mut self, save_path: PathBuf) {
        println!("{} {}fetching util files", style("[5/6]").bold().dim(), BOX);
        let url = self.file_url.clone();
        futures::stream::iter(self.files.util_list.clone().into_iter().map(|util| {
            let url_cloned = url.clone();

            let file_name = PathBuf::from(&util.filename);
            let file = file_name.file_name().unwrap().to_string_lossy().to_string();
            let path = save_path.join("utils").join(&file);

            async move {
                if !path.exists() {
                    match request_file(format!("{}/{}", &url_cloned, &util.filename)).await {
                        Ok(res) => {
                            let bytes = res
                                .bytes()
                                .await
                                .expect("Could not convert to bytes!")
                                .to_vec();

                            write_to_file(&path, &bytes).await.unwrap();

                            log::info!("[✔] Fetched {}", util.filename);
                        }
                        Err(why) => {
                            log::warn!("[❌] Could not fetch {}, {}", util.filename, why);
                        }
                    };
                } else {
                    log::info!("Skipping {:#?}", &file);
                }
            }
        }))
        .buffer_unordered(self.max_concurrent_downloads)
        .collect::<Vec<()>>()
        .await;

        println!(
            "{} {}fetched {} util files",
            style("[6/6]").bold().dim(),
            BOX,
            &self.files.util_list.len(),
        );
    }
}

async fn request_file<T: AsRef<str>>(url: T) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    let res = client
        .get(url.as_ref())
        .header("User-Agent", "KingsIsle Patcher");

    res.send().await
}

async fn write_to_file(path: &PathBuf, content: &Vec<u8>) -> std::io::Result<()> {
    tokio::fs::create_dir_all(&path.parent().unwrap()).await?;
    tokio::fs::File::create(&path).await?;
    tokio::fs::write(&path, content).await?;

    Ok(())
}

//? I'm Crazy HAHAHAHA 🥰😎🤓🤠😩
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LatestFiles {
    latest_file_list: HashMap<String, LatestFileList>,
}

//? Crazy?
#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct LatestFileList {
    record: RecordUnion,
}

//? I was crazy once.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RecordUnion {
    PurpleRecord(PurpleRecord),
    RecordElementArray(Vec<RecordElement>),
}

//? The locked me in a room.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RecordElement {
    #[serde(rename = "CRC")]
    crc: Option<Crc>,
    compressed_header_size: Option<Crc>,
    file_type: Option<Crc>,
    #[serde(rename = "HeaderCRC")]
    header_crc: Option<Crc>,
    header_size: Option<Crc>,
    size: Option<Crc>,
    src_file_name: Option<Name>,
    tar_file_name: Option<Name>,
    name: Option<Name>,
}

//? A rubber room.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Crc {
    #[serde(rename = "#text")]
    value: i64,
    #[serde(rename = "@TYPE")]
    type_: CrcType,
}

//? A rubber room with rats.
#[derive(Debug, Deserialize)]
pub enum CrcType {
    #[serde(rename = "UINT")]
    Uint,
}

//? And rats make me crazy.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Name {
    #[serde(rename = "#text")]
    value: Option<String>,
    #[serde(rename = "@TYPE")]
    type_: SrcFileNameType,
}

#[derive(Debug, Deserialize)]
pub enum SrcFileNameType {
    #[serde(rename = "STR")]
    Str,
}

//? Crazy?
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PurpleRecord {
    version: Option<Crc>,
    #[serde(rename = "CRC")]
    crc: Option<Crc>,
    compressed_header_size: Option<Crc>,
    file_type: Option<Crc>,
    #[serde(rename = "HeaderCRC")]
    header_crc: Option<Crc>,
    header_size: Option<Crc>,
    size: Option<Crc>,
    src_file_name: Option<Name>,
    tar_file_name: Option<TarFileName>,
}

//? I was crazy once.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct TarFileName {
    #[serde(rename = "@TYPE")]
    tar_file_name_type: SrcFileNameType,
}
