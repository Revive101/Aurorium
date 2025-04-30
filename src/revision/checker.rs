use super::Revision;
use crate::errors::RevisionError;
use std::io;

pub struct Checker {
    concurrent_downloads: usize,
}

impl Checker {
    pub async fn new(concurrent_downloads: usize) -> io::Result<Self> {
        Ok(Self { concurrent_downloads })
    }

    pub async fn check_revision(&mut self) -> Result<(), RevisionError> {
        let revision = Revision::check().await?;

        // Check if the Revision is already fetched
        // if not, fetch it & add it somehow to the list

        Ok(())
    }
}
