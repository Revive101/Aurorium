use crate::{
    errors::DbError,
    revision::{Asset, Revision},
};
use async_sqlite::{Client, ClientBuilder, JournalMode};
use rusqlite::{Connection, OptionalExtension, params};
use rusqlite_migration::{M, Migrations};
use std::sync::LazyLock;
use tracing::info;

static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::new(vec![M::up(
        "
            CREATE TABLE revisions (
                revision_name TEXT NOT NULL PRIMARY KEY,
                number INTEGER NOT NULL
            );

            CREATE TABLE assets (
                revision TEXT NOT NULL REFERENCES revisions(revision_name),
                file_name TEXT NOT NULL,
                file_type INTEGER NOT NULL,
                size INTEGER NOT NULL,
                crc INTEGER NOT NULL,
                header_crc INTEGER NOT NULL,
                header_size INTEGER NOT NULL,
                compressed_header_size INTEGER NOT NULL,

                origin_revision TEXT NOT NULL REFERENCES revisions(revision_name),

                PRIMARY KEY (revision, file_name)
            );

            CREATE INDEX idx_assets_lookup ON assets (file_name, crc, size);
        ",
    )])
});

#[derive(Clone)]
pub struct Database {
    client: Client,
}

impl Database {
    pub async fn init(path: &str) -> miette::Result<Self> {
        let client = ClientBuilder::new()
            .path(path)
            .journal_mode(JournalMode::Wal)
            .open()
            .await
            .map_err(DbError::AsyncSqlite)?;

        client
            .conn_mut_and_then(|conn: &mut Connection| -> Result<(), DbError> {
                conn.pragma_update(None, "foreign_keys", true)?;
                MIGRATIONS.to_latest(conn).map_err(DbError::Migration)?;
                Ok(())
            })
            .await?;

        info!("Database {} initialized!", path);

        Ok(Self { client })
    }

    pub async fn insert_revision(&self, revision_name: String, number: i64) -> miette::Result<()> {
        if number < 0 {
            return Err(DbError::InvalidRevisionNumber(number))?;
        }

        self.client
            .conn_and_then(move |conn| -> Result<(), DbError> {
                conn.execute(
                    "INSERT INTO revisions (revision_name, number) VALUES (?1, ?2)",
                    params![revision_name, number],
                )?;
                Ok(())
            })
            .await?;

        Ok(())
    }

    pub async fn get_latest_revision(&self) -> miette::Result<Option<Revision>> {
        let revision = self
            .client
            .conn_and_then(|conn| -> Result<Option<Revision>, DbError> {
                conn.query_row(
                    "SELECT revision_name, number FROM revisions ORDER BY number DESC LIMIT 1",
                    [],
                    |row| {
                        Ok(Revision {
                            name: row.get(0)?,
                            number: row.get(1)?,
                        })
                    },
                )
                .optional()
                .map_err(Into::into)
            })
            .await?;

        Ok(revision)
    }

    pub async fn list_revisions(&self) -> miette::Result<Vec<String>> {
        let revisions = self
            .client
            .conn_and_then(|conn| -> Result<Vec<String>, DbError> {
                let mut stmt =
                    conn.prepare("SELECT revision_name FROM revisions ORDER BY number DESC")?;
                let names = stmt
                    .query_map([], |row| row.get(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(names)
            })
            .await?;

        Ok(revisions)
    }

    pub async fn latest_revision(&self) -> miette::Result<Option<Revision>> {
        let latest_revision = self
            .client
            .conn_and_then(|conn| -> Result<Option<Revision>, DbError> {
                let mut stmt = conn.prepare(
                    "SELECT revision_name, number FROM revisions ORDER BY number DESC LIMIT 1",
                )?;
                let revision = stmt
                    .query_row([], |row| {
                        Ok(Revision {
                            name: row.get(0)?,
                            number: row.get(1)?,
                        })
                    })
                    .optional()?;
                Ok(revision)
            })
            .await?;

        Ok(latest_revision)
    }

    pub async fn insert_new_revision(
        &self,
        revision: Revision,
        fetched_assets: Vec<Asset>,
    ) -> miette::Result<Vec<Asset>> {
        let assets_to_download = self
        .client
        .conn_mut_and_then(move |conn: &mut Connection| -> Result<Vec<Asset>, DbError> {
            let tx = conn.transaction().map_err(DbError::Transaction)?;

            tx.execute(
                "INSERT OR IGNORE INTO revisions (revision_name, number) VALUES (?1, ?2)",
                params![revision.name, revision.number],
            )?;

            let mut stmt_check = tx.prepare(
                "SELECT origin_revision FROM assets WHERE file_name = ?1 AND crc = ?2 AND size = ?3 LIMIT 1"
            )?;
            let mut stmt_insert = tx.prepare(
                "INSERT OR IGNORE INTO assets (
                    revision, file_name, file_type, size, crc,
                    header_crc, header_size, compressed_header_size, origin_revision
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;

            let mut assets_to_download = Vec::new();

            for asset in fetched_assets {
                let existing_origin: Option<String> = stmt_check
                    .query_row(params![asset.file_name, asset.crc, asset.size], |row| row.get(0))
                    .optional()?;

                let origin_revision = match existing_origin {
                    Some(origin) => {
                        if origin == revision.name {
                            assets_to_download.push(asset.clone());
                        }
                        origin
                    }
                    None => {
                        assets_to_download.push(asset.clone());
                        revision.name.clone()
                    }
                };

                stmt_insert.execute(params![
                    revision.name,
                    asset.file_name,
                    asset.file_type,
                    asset.size,
                    asset.crc,
                    asset.header_crc,
                    asset.header_size,
                    asset.compressed_header_size,
                    origin_revision
                ])?;
            }

            drop(stmt_check);
            drop(stmt_insert);
            tx.commit().map_err(DbError::Transaction)?;

            Ok(assets_to_download)
        })
        .await?;

        Ok(assets_to_download)
    }

    pub async fn get_revision_for_asset(
        &self,
        revision_name: String,
        file_name: String,
    ) -> Result<Option<String>, DbError> {
        let result = self
            .client
            .conn_and_then(move |conn| -> Result<Option<String>, DbError> {
                let mut stmt = conn.prepare(
                    "SELECT origin_revision FROM assets WHERE revision = ?1 AND file_name = ?2 LIMIT 1",
                )?;

                let asset_info = stmt
                    .query_row(params![revision_name, file_name], |row| {
                        let origin_revision: String = row.get(0)?;
                        Ok(origin_revision)
                    })
                    .optional()?;

                Ok(asset_info)
            })
            .await?;

        Ok(result)
    }
}
