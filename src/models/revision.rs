use super::asset::AssetList;
use crate::{
    ARGS, REVISIONS,
    xml_parser::{parse_xml, sanitize_content},
};
use regex::Regex;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize)]
pub struct LocalRevision {
    /// Revision name (e.g., V_r773351.Wizard_1_570_0_Live)
    pub name: String,

    /// Numeric revision number for comparison
    pub revision_number: u64,

    /// Path where this revision is stored
    pub path: PathBuf,

    /// List of assets in this revision
    pub assets: AssetList,
}

impl LocalRevision {
    pub async fn new<P>(name: &str, base_path: P, assets: AssetList) -> Option<Self>
    where
        P: AsRef<Path>,
    {
        let path = base_path.as_ref().join(name);
        Some(Self {
            name: name.to_string(),
            revision_number: Self::capture_revision(&name)?,
            assets,
            path,
        })
    }

    /// Load revision from name
    pub async fn from_name<P>(name: &str, base_path: P) -> Option<Self>
    where
        P: AsRef<Path>,
    {
        let path = base_path.as_ref().join(name);

        if path.exists() {
            return Some(Self {
                name: name.to_string(),
                revision_number: Self::capture_revision(name)?,
                assets: Self::generate_asset_list(&path).await,
                path,
            });
        }

        None
    }

    pub async fn newest() -> Option<Self> {
        let revisions = REVISIONS.read().await;
        revisions.iter().max_by_key(|rev| rev.revision_number).cloned()
    }

    pub async fn init_all<P>(base_path: P) -> std::io::Result<()>
    where
        P: AsRef<Path> + Copy,
    {
        println!("Initializing all revisions on disk...");
        let mut revisions = REVISIONS.write().await;

        let path = base_path.as_ref();

        if let Ok(mut entries) = tokio::fs::read_dir(path).await {
            while let Some(entry) = entries.next_entry().await? {
                if !entry.file_type().await.unwrap().is_dir() {
                    continue;
                }

                let name = entry.file_name().to_string_lossy().to_string();
                let revision = Self::from_name(&name, base_path).await.unwrap();
                revisions.push(revision);
            }
        }

        Ok(())
    }

    pub async fn find_revision_for_asset(revision: String, asset_name: &String) -> Option<String> {
        let local_revision = Self::from_name(&revision, &ARGS.save_directory).await?;
        let local_asset = local_revision.assets.find_by_name(&asset_name)?;

        let mut revisions = REVISIONS.read().await.clone();
        revisions.sort_by_key(|r| r.revision_number);

        for rev in revisions {
            for asset in rev.assets.all() {
                if asset.crc == local_asset.crc && asset.size == local_asset.size {
                    return Some(rev.name);
                }
            }
        }

        None
    }

    fn capture_revision(name: &str) -> Option<u64> {
        let revision_regex = Regex::new(r"^V_r(\d+)\.Wizard.*$").ok()?;

        // Extract revision number using regex
        let cap = revision_regex.captures(name)?;
        let revision_number = cap.get(1)?.as_str().parse::<u64>().ok()?;

        Some(revision_number)
    }

    async fn generate_asset_list(path: &PathBuf) -> AssetList {
        let path = path.join("LatestFileList.xml");

        if !path.exists() {
            return AssetList::default();
        }

        let mut list = AssetList::default();
        let xml_content = fs::read_to_string(path).await.unwrap();
        let (wads, utils) = parse_xml(&sanitize_content(&xml_content).await.unwrap()).unwrap();

        list.wads = wads;
        list.utils = utils;

        list
    }
}

impl PartialEq for LocalRevision {
    fn eq(&self, other: &Self) -> bool {
        self.revision_number == other.revision_number
    }
}

impl Eq for LocalRevision {}
