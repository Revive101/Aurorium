use crate::utils::{Endianness, hex_decode};
use anyhow::{Context, anyhow};
use regex::Regex;
use std::io::Cursor;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const BUFFER_SIZE: usize = 256;
const SESSION_ACCEPT: &str = "0DF02700000000000802220000000000000000000000000000000000000000000000000000000000000000";
const SERVICE_ID: u8 = 8; // PATCH
const MESSAGE_ID: u8 = 2; // MSG_LATEST_FILE_LIST_V2

trait WizIntegration {
    const FOOD_HEADER: [u8; 2] = [0x0D, 0xF0];

    fn read_bytestring(&mut self) -> impl Future<Output = anyhow::Result<String>> + Send;
    fn verify_food_header(&mut self) -> impl Future<Output = anyhow::Result<bool>> + Send;
}

impl WizIntegration for Cursor<[u8; BUFFER_SIZE]> {
    const FOOD_HEADER: [u8; 2] = [0x0D, 0xF0];

    async fn read_bytestring(&mut self) -> anyhow::Result<String> {
        let len = self.read_u16_le().await.context("Failed to read bytestring length")?;
        let mut buffer = vec![0u8; len as usize];
        tokio::io::AsyncReadExt::read_exact(self, &mut buffer)
            .await
            .context("Failed to read bytestring data")?;
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    async fn verify_food_header(&mut self) -> anyhow::Result<bool> {
        let food_header = self.read_u16_le().await.context("Failed to read FOOD header")?;
        Ok(food_header.to_le_bytes() == Self::FOOD_HEADER)
    }
}

#[derive(Debug)]
pub struct PatchInfo {
    pub list_file_url: String,
    pub url_prefix: String,
    pub revision: String,
}

impl PatchInfo {
    pub async fn fetch_latest(host: &str, port: &str) -> anyhow::Result<Self> {
        let mut stream = TcpStream::connect(format!("{host}:{port}"))
            .await
            .with_context(|| format!("Unable to connect to {host}:{port}"))?;
        let mut buffer = [0u8; BUFFER_SIZE];

        // read initial offer
        let bytes_read = stream.read(&mut buffer).await.context("Failed to read initial offer from server")?;
        if bytes_read == 0 {
            return Err(anyhow!("Server closed connection unexpectedly"));
        }

        // Send session accept
        let accept_bytes = hex_decode(SESSION_ACCEPT, &Endianness::Little).ok_or_else(|| anyhow!("Failed to parse hex string to bytes"))?;

        stream.write_all(&accept_bytes).await.context("Failed to send session accept")?;

        // Read server response
        let bytes_read = stream.read(&mut buffer).await.context("Failed to read server response")?;
        if bytes_read == 0 {
            return Err(anyhow!("Server closed connection after session accept"));
        }

        stream.shutdown().await.context("Failed to shutdown connection")?;

        Self::parse_response(&buffer).await
    }

    async fn parse_response(buffer: &[u8; BUFFER_SIZE]) -> anyhow::Result<Self> {
        let mut cursor: Cursor<[u8; BUFFER_SIZE]> = Cursor::new(*buffer);

        if !cursor.verify_food_header().await? {
            return Err(anyhow!("Invalid Header sequence for buffer:\n{:?}", cursor.into_inner()));
        }

        // Skip unused fields
        let _content_length = cursor.read_u16_le().await.context("Failed to read content length")?;
        let _ = cursor.read_u32_le().await.context("Failed to read padding field")?;
        /*
            isControl u8
            opCode u8
            Padding u16
        */

        // Verify protocol headers
        let service_id = cursor.read_u8().await.context("Failed to read serviceID")?;
        let message_id = cursor.read_u8().await.context("Failed to read messageID")?;

        if service_id != SERVICE_ID || message_id != MESSAGE_ID {
            return Err(anyhow!("Expected SERVICE_ID=8 & MESSAGE_ID=2 but got {service_id} & {message_id}"));
        }

        let _dml_length = cursor.read_u16_le().await.context("Failed to read DML length")?;
        let _latest_version = cursor.read_u32_le().await.context("Failed to read latest version")?;
        let _list_file_name = cursor.read_bytestring().await.context("Failed to read list file name")?;
        let _ = cursor.read_u128_le().await.context("Failed to read file metadata")?;
        /*
            ListFileType u32
            ListFileTime u32
            ListFileSize u32
            ListFileCRC  u32
        */
        let list_file_url = cursor.read_bytestring().await.context("Failed to read list file URL")?;
        let url_prefix = cursor.read_bytestring().await.context("Failed to read URL prefix")?;

        let revision = Self::parse_revision(&list_file_url).context("Failed to parse revision from URL")?;
        Ok(PatchInfo {
            list_file_url: list_file_url.clone(),
            url_prefix,
            revision: revision,
        })
    }

    fn parse_revision(url: &str) -> anyhow::Result<String> {
        let reg = Regex::new(r"/(V_[^/]+)/")?;

        if let Some(cap) = reg.captures(url).and_then(|c| c.get(1)) {
            return Ok(cap.as_str().to_string());
        }

        return Err(anyhow!("Failed to parse revision string: {url}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: &str = "patch.us.wizard101.com";
    const PORT: &str = "12500";

    #[tokio::test]
    async fn test_connection() {
        let patch_info = PatchInfo::fetch_latest(HOST, PORT).await.unwrap();
        assert!(!patch_info.list_file_url.is_empty(), "List file URL should not be empty");
        assert!(!patch_info.url_prefix.is_empty(), "URL prefix should not be empty");
        assert!(!patch_info.revision.is_empty(), "Revision should not be empty");
    }

    #[tokio::test]
    async fn test_parse_revision_valid() {
        let url = "http://versionak.us.wizard101.com/WizPatcher/V_r778979.Wizard_1_580_0_Live/Windows/LatestFileList.bin";
        let result = PatchInfo::parse_revision(url).unwrap();

        assert_eq!(result, "V_r778979.Wizard_1_580_0_Live");
    }
}
