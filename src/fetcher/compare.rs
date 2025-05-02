use crate::models::{asset::Asset, revision::LocalRevision};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum RevisionDiffError {
    #[error("New revision has no assets (failed to parse?)")]
    NoAssets,
}

#[derive(Debug, Default)]
pub struct RevisionDiff {
    /// New assets that don't exist in the previous revision
    pub new_assets: Vec<Asset>,

    /// Assets that exist in both revisions but have changed
    pub changed_assets: Vec<Asset>,

    /// Assets that exist in both revisions and are unchanged
    pub unchanged_assets: Vec<Asset>,

    /// Assets that existed in the previous revision but not in the new one
    pub removed_assets: Vec<Asset>,
}

impl RevisionDiff {
    /// Get all assets that need to be downloaded (new + changed)
    pub fn assets_to_download(&self) -> Vec<&Asset> {
        let mut result = Vec::with_capacity(self.new_assets.len() + self.changed_assets.len());
        result.extend(self.new_assets.iter());
        result.extend(self.changed_assets.iter());
        result
    }
}

pub fn compare_revisions(new_revision: &LocalRevision, old_revision: &Option<LocalRevision>) -> Result<RevisionDiff, RevisionDiffError> {
    if new_revision.assets.is_empty() {
        return Err(RevisionDiffError::NoAssets);
    }

    let mut diff = RevisionDiff::default();

    // If there's no old asset list, all assets are new
    if old_revision.is_none() {
        diff.new_assets = new_revision.assets.all().cloned().collect();
        return Ok(diff);
    }

    let old_assets = &old_revision.as_ref().unwrap().assets;

    // Create a map of old assets by filename for quick lookup
    let mut old_asset_map: HashMap<String, &Asset> = HashMap::new();
    for asset in old_assets.all() {
        old_asset_map.insert(asset.filename.clone(), asset);
    }

    // Compare new assets to old ones
    for asset in new_revision.assets.all() {
        if let Some(old_asset) = old_asset_map.get(&asset.filename) {
            // Asset exists in both revisions
            println!("{} == {} ? {}", old_asset.crc, asset.crc, old_asset.crc == asset.crc);
            if asset.crc == old_asset.crc && asset.size == old_asset.size {
                diff.unchanged_assets.push(asset.clone());
            } else {
                diff.changed_assets.push(asset.clone());
            }

            // Remove from map to track what's left at the end
            old_asset_map.remove(&asset.filename);
        } else {
            // Asset is new
            diff.new_assets.push(asset.clone());
        }
    }

    // Any assets left in the map were removed in the new revision
    for (_, asset) in old_asset_map {
        diff.removed_assets.push(asset.clone());
    }

    Ok(diff)
}
