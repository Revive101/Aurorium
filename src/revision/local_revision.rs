use miette::Severity;
use regex::Regex;
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crate::revision::asset_list::AssetList;

static REVISION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^V_r(\d+)\.Wizard.*$").unwrap());

#[derive(Debug)]
pub struct LocalRevision {
    /// Revision name (e.g., `V_r773351.Wizard_1_570_0_Live`)
    pub name: String,

    /// Numeric revision number for comparison (e.g., `773351`)
    pub number: u64,

    /// Path where this revision is stored
    pub path: PathBuf,

    /// List of assets in this revision
    pub assets: AssetList,
}

impl LocalRevision {
    pub fn new<P>(name: &str, base_path: P, assets: AssetList) -> miette::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            name: name.to_string(),
            number: Self::extract_revision_number(name)?,
            path: base_path.as_ref().join(name),
            assets,
        })
    }

    pub async fn load_from_name<P>(name: &str, base_path: P) -> miette::Result<Self> {
        todo!()
    }

    pub async fn latest() -> miette::Result<Self> {
        todo!()
    }

    pub async fn init_all<P>(base_path: P) -> miette::Result<()> {
        todo!()
    }

    // Internal functions
    fn extract_revision_number(name: &str) -> miette::Result<u64> {
        if let Some(captures) = REVISION_RE.captures(name).and_then(|c| c.get(1)) {
            let revision_number = captures.as_str().parse::<u64>().map_err(|e| {
                miette::miette!(
                    "Failed to parse revision number from '{}': {}",
                    name,
                    e.to_string()
                )
            })?;

            return Ok(revision_number);
        }

        Err(miette::miette!(
            severity = Severity::Error,
            "Failed to extract revision number from '{}'",
            name
        ))
    }
}
