use std::{iter::Chain, slice::Iter};

/// Represents a single WAD file with its metadata
#[derive(Debug, Clone, Default)]
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
pub struct AssetList {
    pub wads: Vec<Asset>,
    pub utils: Vec<Asset>,
}

impl AssetList {
    /// Returns a combined list of all assets
    pub fn all(&self) -> Chain<Iter<'_, Asset>, Iter<'_, Asset>> {
        self.wads.iter().chain(self.utils.iter())
    }

    /// Compare self vs other, returning the assets that are new or changed.
    pub fn diff<'a>(&self, other: &'a AssetList) -> Vec<&'a Asset> {
        let mut changed = Vec::new();

        for new_asset in other.all() {
            match self.all().find(|old| old.filename == new_asset.filename) {
                Some(old) if old.crc == new_asset.crc && old.size == new_asset.size => { /* unchanged */ }
                _ => changed.push(new_asset),
            }
        }
        changed
    }

    pub fn is_empty(&self) -> bool {
        self.wads.is_empty() && self.utils.is_empty()
    }
}
