use regex::Regex;
use std::{path::PathBuf, sync::LazyLock};

use crate::revision::asset_list::AssetList;

static REVISION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^V_r(\d+)\.Wizard.*$").unwrap());

#[derive(Debug)]
pub struct LocalRevision {
    /// Full Revision-name (e.g. "V_r773351.Wizard_1_570_0_Live")
    pub name: String,

    /// Numeric revision number for comparison (e.g. 773351)
    pub number: u64,

    /// Path where this revision is stored
    pub path: PathBuf,

    /// List of assets in this revision
    pub assets: AssetList,
}

impl LocalRevision {
    pub fn new<P>(name: &str, base_path: P, assets: AssetList) -> Option<Self> {
        todo!()
    }

    pub async fn load_from_name<P>(name: &str, base_path: P) -> Option<Self> {
        todo!()
    }

    pub async fn latest() -> Option<Self> {
        todo!()
    }

    pub async fn init_all<P>(base_path: P) -> miette::Result<()> {
        todo!()
    }

    // Internal functions
    fn extract_revision_number(name: &str) -> Option<u64> {
        let captures = REVISION_RE.captures(name)?;
        let revision_number = captures.get(1)?.as_str().parse::<u64>().ok()?;

        Some(revision_number)
    }
}
