#[derive(Debug, Default)]
pub struct Asset {
    pub file_name: String,
    pub file_type: String,
    pub size: u64,
    pub header_size: u64,
    pub compressed_header_size: u64,
    pub crc: u32,
    pub header_crc: u32,
}

#[derive(Debug, Default)]
pub struct AssetList {
    pub wads: Vec<Asset>,
    pub utils: Vec<Asset>,
}

impl AssetList {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.wads.is_empty() && self.utils.is_empty()
    }
}
