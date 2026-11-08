use crate::{
    errors::WizardPatcherError,
    revision::Revision,
    utils::{Endianness, hex_decode},
};
use regex::Regex;
use std::{io::Cursor, sync::LazyLock, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::info;

static LIST_URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/(V_[^/]+)/").unwrap());
static REVISION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^V_r(\d+)\.Wizard.*$").unwrap());

const FRAME_HEADER_LENGTH: usize = 4;
const SESSION_OFFER_LENGTH: usize = 28;
const SESSION_ACCEPT: &str =
    "0DF02700000000000802220000000000000000000000000000000000000000000000000000000000000000"; // PatchMessages(8) -> MSG_LATEST_FILE_LIST_V2 (2)
const TCP_TIMEOUT: Duration = Duration::from_secs(30);

trait WizIntegration {
    const FOOD_HEADER: [u8; 2] = [0x0D, 0xF0];

    fn read_bytestring(&mut self) -> impl Future<Output = miette::Result<String>> + Send;
    fn verify_food_header(&mut self) -> impl Future<Output = miette::Result<bool>> + Send;
    fn read_le(&mut self, size: usize) -> impl Future<Output = miette::Result<Vec<u8>>> + Send;
}

impl WizIntegration for Cursor<&[u8]> {
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
        let mut stream =
            tokio::time::timeout(TCP_TIMEOUT, TcpStream::connect(format!("{host}:{port}")))
                .await
                .map_err(|_| WizardPatcherError::TimeoutError)?
                .map_err(WizardPatcherError::ConnectionError)?;

        info!("Connected to the PatchServer at {host}:{port}");

        // Read the exact 28-byte initial offer from the server
        let mut offer_buffer = [0u8; SESSION_OFFER_LENGTH];
        tokio::time::timeout(TCP_TIMEOUT, stream.read_exact(&mut offer_buffer))
            .await
            .map_err(|_| WizardPatcherError::TimeoutError)?
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::UnexpectedEof => WizardPatcherError::UnexpectedEofError(e),
                _ => WizardPatcherError::ReadError(e),
            })?;

        let offer_packet_len =
            FRAME_HEADER_LENGTH + u16::from_le_bytes([offer_buffer[2], offer_buffer[3]]) as usize;
        if offer_packet_len != SESSION_OFFER_LENGTH {
            Err(WizardPatcherError::UnexpectedResponseLength(
                offer_packet_len,
                SESSION_OFFER_LENGTH,
            ))?;
        }

        // Send our SESSION_ACCEPT packet to the server
        let session_accept_bytes =
            hex_decode(SESSION_ACCEPT, &Endianness::Little).expect("This should not fail!");
        tokio::time::timeout(TCP_TIMEOUT, stream.write_all(&session_accept_bytes))
            .await
            .map_err(|_| WizardPatcherError::TimeoutError)?
            .map_err(WizardPatcherError::WriteError)?;

        // Read the 4-byte frame header (2-byte FOOD magic + 2-byte little-endian payload length)
        let mut frame_header = [0u8; FRAME_HEADER_LENGTH];
        tokio::time::timeout(TCP_TIMEOUT, stream.read_exact(&mut frame_header))
            .await
            .map_err(|_| WizardPatcherError::TimeoutError)?
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::UnexpectedEof => WizardPatcherError::UnexpectedEofError(e),
                _ => WizardPatcherError::ReadError(e),
            })?;

        let payload_len = u16::from_le_bytes([frame_header[2], frame_header[3]]) as usize;
        let mut response_buffer = vec![0u8; FRAME_HEADER_LENGTH + payload_len];
        response_buffer[..FRAME_HEADER_LENGTH].copy_from_slice(&frame_header);

        // Read the full framed response payload
        tokio::time::timeout(
            TCP_TIMEOUT,
            stream.read_exact(&mut response_buffer[FRAME_HEADER_LENGTH..]),
        )
        .await
        .map_err(|_| WizardPatcherError::TimeoutError)?
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::UnexpectedEof => WizardPatcherError::UnexpectedEofError(e),
            _ => WizardPatcherError::ReadError(e),
        })?;

        let _ = tokio::time::timeout(TCP_TIMEOUT, stream.shutdown()).await;
        Self::parse_response(&response_buffer).await
    }

    async fn parse_response(buffer: &[u8]) -> miette::Result<Self> {
        let mut cursor = Cursor::new(buffer);

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

        Err(WizardPatcherError::RevisionParseError(url.to_string()))?
    }

    fn extract_revision_number(name: &str) -> miette::Result<i64> {
        if let Some(captures) = REVISION_RE.captures(name).and_then(|c| c.get(1)) {
            let revision_number = captures
                .as_str()
                .parse::<i64>()
                .map_err(|_| WizardPatcherError::RevisionParseError(name.to_string()))?;

            if revision_number < 0 {
                Err(WizardPatcherError::InvalidRevisionNumber(revision_number))?;
            }

            return Ok(revision_number);
        }

        Err(WizardPatcherError::RevisionParseError(name.to_string()))?
    }
}
