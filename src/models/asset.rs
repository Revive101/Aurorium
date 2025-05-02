use std::{iter::Chain, slice::Iter};

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

impl Asset {
    /// Checks if this asset has the same content as another
    pub fn same_content(&self, other: &Asset) -> bool {
        self.crc == other.crc && self.size == other.size
    }
}

/// Categorizes different types of game assets
#[derive(Debug, Default)]
pub struct AssetList {
    pub wads: Vec<Asset>,
    pub utils: Vec<Asset>,
}

impl AssetList {
    /// Returns a combined list of all assets
    pub fn assets(&self) -> Chain<Iter<'_, Asset>, Iter<'_, Asset>> {
        self.wads.iter().chain(self.utils.iter())
    }
}
