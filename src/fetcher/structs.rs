use serde::Deserialize;
use std::collections::HashMap;

pub trait Element {
    fn get_filename(&self) -> Option<String>;
    fn get_size(&self) -> i64;
    fn get_header_size(&self) -> i64;
    fn get_compressed_header_size(&self) -> i64;
    fn get_crc(&self) -> i64;
    fn get_header_crc(&self) -> i64;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LatestFiles {
    pub latest_file_list: HashMap<String, LatestFileList>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct LatestFileList {
    pub record: RecordUnion,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RecordUnion {
    PurpleRecord(PurpleRecord),
    RecordElementArray(Vec<RecordElement>),
}

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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Crc {
    #[serde(rename = "#text")]
    value: i64,
    #[serde(rename = "@TYPE")]
    type_: CrcType,
}

#[derive(Debug, Deserialize)]
pub enum CrcType {
    #[serde(rename = "UINT")]
    Uint,
}

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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct TarFileName {
    #[serde(rename = "@TYPE")]
    tar_file_name_type: SrcFileNameType,
}

impl Element for PurpleRecord {
    fn get_compressed_header_size(&self) -> i64 {
        self.compressed_header_size.as_ref().unwrap().value
    }
    fn get_crc(&self) -> i64 {
        self.crc.as_ref().unwrap().value
    }
    fn get_filename(&self) -> Option<String> {
        match &self.src_file_name {
            Some(file) => file.value.clone(),
            None => None,
        }
    }
    fn get_header_crc(&self) -> i64 {
        self.header_crc.as_ref().unwrap().value
    }
    fn get_header_size(&self) -> i64 {
        self.header_size.as_ref().unwrap().value
    }
    fn get_size(&self) -> i64 {
        self.size.as_ref().unwrap().value
    }
}

impl Element for RecordElement {
    fn get_compressed_header_size(&self) -> i64 {
        self.compressed_header_size.as_ref().unwrap().value
    }
    fn get_crc(&self) -> i64 {
        self.crc.as_ref().unwrap().value
    }
    fn get_filename(&self) -> Option<String> {
        match &self.src_file_name {
            Some(file) => file.value.clone(),
            None => None,
        }
    }
    fn get_header_crc(&self) -> i64 {
        self.header_crc.as_ref().unwrap().value
    }
    fn get_header_size(&self) -> i64 {
        self.header_size.as_ref().unwrap().value
    }
    fn get_size(&self) -> i64 {
        self.size.as_ref().unwrap().value
    }
}
