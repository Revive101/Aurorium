use crate::{
    REVISIONS,
    models::{asset::Asset, revision::LocalRevision},
};
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

pub async fn compare_revisions(
    new_revision: &LocalRevision,
    old_revision: Option<LocalRevision>,
) -> Result<RevisionDiff, RevisionDiffError> {
    if new_revision.assets.is_empty() {
        return Err(RevisionDiffError::NoAssets);
    }

    let mut diff = RevisionDiff::default();

    // If there's no old asset list, all assets are new
    if old_revision.is_none() {
        diff.new_assets = new_revision.assets.all().cloned().collect();
        return Ok(diff);
    }

    let mut old_revision = old_revision.unwrap();

    let new_revision_number = new_revision.revision_number;
    let old_revision_number = old_revision.revision_number;

    if new_revision_number == old_revision_number {
        // If the revisions are the same, we check if there are missing assets (eg. if the fetching was interrupted)

        // we want to use the newest revision except the one we are currently using
        let updated_revision = REVISIONS.read().await.clone();

        if updated_revision.len() == 1 {
            diff.new_assets = new_revision.assets.all().cloned().collect();
            return Ok(diff);
        }

        old_revision = updated_revision
            .iter()
            .filter(|rev| rev.revision_number != new_revision_number)
            .max_by_key(|rev| rev.revision_number)
            .cloned()
            .unwrap();
    }

    // Create a map of old assets by filename for quick lookup
    let mut old_asset_map: HashMap<String, &Asset> = HashMap::new();
    for asset in old_revision.assets.all() {
        old_asset_map.insert(asset.filename.clone(), asset);
    }

    // Compare new assets to old ones
    for asset in new_revision.assets.all() {
        if let Some(old_asset) = old_asset_map.get(&asset.filename) {
            // Asset exists in both revisions
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
