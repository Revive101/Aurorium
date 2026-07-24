use crate::{
    errors::WizardPatcherError,
    revision::Revision,
    utils::{Endianness, hex_decode},
};
use regex::Regex;
use std::{io::Cursor, sync::LazyLock};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::info;

static LIST_URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/(V_[^/]+)/").unwrap());
static REVISION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^V_r(\d+)\.Wizard.*$").unwrap());

const BUFFER_SIZE: usize = 256;
const SESSION_OFFER_LENGTH: usize = 28;
const SESSION_ACCEPT: &str =
    "0DF02700000000000802220000000000000000000000000000000000000000000000000000000000000000"; // PatchMessages(8) -> MSG_LATEST_FILE_LIST_V2 (2)

trait WizIntegration {
    const FOOD_HEADER: [u8; 2] = [0x0D, 0xF0];

    fn read_bytestring(&mut self) -> impl Future<Output = miette::Result<String>> + Send;
    fn verify_food_header(&mut self) -> impl Future<Output = miette::Result<bool>> + Send;
    fn read_le(&mut self, size: usize) -> impl Future<Output = miette::Result<Vec<u8>>> + Send;
}

impl WizIntegration for Cursor<[u8; BUFFER_SIZE]> {
    const FOOD_HEADER: [u8; 2] = [0x0D, 0xF0];

    async fn read_bytestring(&mut self) -> miette::Result<String> {
        let len = self
            .read_u16_le()
            .await
            .map_err(WizardPatcherError::UnexpectedEofError)?;
        let mut buffer = vec![0u8; len as usize];
        tokio::io::AsyncReadExt::read_exact(self, &mut buffer)
            .await
            .map_err(WizardPatcherError::UnexpectedEofError)?;
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    async fn verify_food_header(&mut self) -> miette::Result<bool> {
        let food_header = self
            .read_u16_le()
            .await
            .map_err(WizardPatcherError::UnexpectedEofError)?;

        Ok(food_header.to_le_bytes() == Self::FOOD_HEADER)
    }

    async fn read_le(&mut self, size: usize) -> miette::Result<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        tokio::io::AsyncReadExt::read_exact(self, &mut buffer)
            .await
            .map_err(WizardPatcherError::UnexpectedEofError)?;
        Ok(buffer)
    }
}

#[derive(Debug, Clone)]
pub struct WizardPatcher {
    pub list_file_url: String,
    pub url_prefix: String,
    pub revision: Revision,
}

impl WizardPatcher {
    #[tracing::instrument(ret, level = "debug")]
    pub async fn check_revision(host: &str, port: &str) -> miette::Result<Self> {
        let mut stream = TcpStream::connect(format!("{host}:{port}"))
            .await
            .map_err(WizardPatcherError::ConnectionError)?;

        info!("Connected to the PatchServer at {host}:{port}");

        let mut buffer = [0u8; BUFFER_SIZE];

        // Read the initial offer from the server
        let bytes_read = stream
            .read(&mut buffer)
            .await
            .map_err(WizardPatcherError::ReadError)?;

        if bytes_read == 0 {
            return Err(miette::miette!(
                "Server closed the connection unexpectedly."
            ));
        }

        // Further checks to ensure the received data is valid
        if bytes_read != SESSION_OFFER_LENGTH {
            return Err(WizardPatcherError::UnexpectedResponseLength(
                bytes_read,
                SESSION_OFFER_LENGTH,
            ))?;
        }

        // Send our SESSION_ACCEPT packet to the server
        let session_accept_bytes = hex_decode(SESSION_ACCEPT, &Endianness::Little).unwrap();
        stream
            .write_all(&session_accept_bytes)
            .await
            .map_err(WizardPatcherError::WriteError)?;

        let mut response_buffer = [0u8; BUFFER_SIZE];
        let response_bytes_read = stream
            .read(&mut response_buffer)
            .await
            .map_err(WizardPatcherError::ReadError)?;

        if response_bytes_read == 0 {
            return Err(miette::miette!(
                "Server closed the connection unexpectedly after sending SESSION_ACCEPT."
            ));
        }

        stream
            .shutdown()
            .await
            .map_err(WizardPatcherError::ShutdownError)?;

        Self::parse_response(&response_buffer).await
    }

    async fn parse_response(buffer: &[u8; BUFFER_SIZE]) -> miette::Result<Self> {
        let mut cursor: Cursor<[u8; BUFFER_SIZE]> = Cursor::new(*buffer);

        if !cursor.verify_food_header().await? {
            return Err(miette::miette!("Invalid FOOD header in server response."));
        }

        let _ = cursor.read_le(14).await?; // Skip irrelevant bytes
        let _list_file_name = cursor.read_bytestring().await?;
        let _ = cursor.read_le(16).await?; // Skip irrelevant bytes

        let list_file_url = cursor.read_bytestring().await?;
        let url_prefix = cursor.read_bytestring().await?;

        Ok(Self {
            revision: Self::capture_revision(&list_file_url)?,
            url_prefix,
            list_file_url,
        })
    }

    fn capture_revision(url: &str) -> miette::Result<Revision> {
        if let Some(captures) = LIST_URL_RE.captures(url).and_then(|c| c.get(1)) {
            let revision_name = captures.as_str().to_string();
            let revision_number = Self::extract_revision_number(&revision_name)?;

            return Ok(Revision {
                name: revision_name,
                number: revision_number,
            });
        }

        return Err(WizardPatcherError::RevisionParseError(url.to_string()))?;
    }

    fn extract_revision_number(name: &str) -> miette::Result<i64> {
        if let Some(captures) = REVISION_RE.captures(name).and_then(|c| c.get(1)) {
            let revision_number = captures
                .as_str()
                .parse::<i64>()
                .map_err(|_| WizardPatcherError::RevisionParseError(name.to_string()))?;

            if revision_number < 0 {
                return Err(WizardPatcherError::InvalidRevisionNumber(revision_number))?;
            }

            return Ok(revision_number);
        }

        return Err(WizardPatcherError::RevisionParseError(name.to_string()))?;
    }
}
