use serde::Serialize;
use std::{iter::Chain, slice::Iter};

/// Represents a single WAD file with its metadata
#[derive(Debug, Clone, Default, Serialize, Hash)]
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
#[derive(Debug, Clone, Default, Serialize, Hash)]
pub struct AssetList {
    pub wads: Vec<Asset>,
    pub utils: Vec<Asset>,
}

impl AssetList {
    /// Returns a combined list of all assets
    pub fn all(&self) -> Chain<Iter<'_, Asset>, Iter<'_, Asset>> {
        self.wads.iter().chain(self.utils.iter())
    }

    #[must_use]
    pub fn find_by_name(&self, name: &str) -> Option<&Asset> {
        self.all().find(|asset| asset.filename == name)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.wads.is_empty() && self.utils.is_empty()
    }
}
