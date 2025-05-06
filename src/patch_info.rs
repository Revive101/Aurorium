use crate::{
    errors::PatchInfoError,
    utils::{Endianness, hex_decode},
};
use regex::Regex;
use std::{io::Cursor, net::ToSocketAddrs};
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

    fn read_bytestring(&mut self) -> impl Future<Output = Result<String, PatchInfoError>> + Send;
    fn verify_food_header(&mut self) -> impl Future<Output = Result<bool, PatchInfoError>> + Send;
}

impl WizIntegration for Cursor<[u8; BUFFER_SIZE]> {
    const FOOD_HEADER: [u8; 2] = [0x0D, 0xF0];

    async fn read_bytestring(&mut self) -> Result<String, PatchInfoError> {
        let len = self.read_u16_le().await?;
        let mut buffer = vec![0u8; len as usize];
        tokio::io::AsyncReadExt::read_exact(self, &mut buffer).await?;
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    async fn verify_food_header(&mut self) -> Result<bool, PatchInfoError> {
        let food_header = self.read_u16_le().await?;
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
    pub async fn fetch_latest(host: &str, port: &str) -> Result<Self, PatchInfoError> {
        let mut ip = format!("{host}:{port}").to_socket_addrs()?;

        let mut stream = TcpStream::connect(&ip.next().ok_or(PatchInfoError::AddrResolve)?).await?;
        let mut buffer = [0u8; BUFFER_SIZE];

        // read initial offer
        #[allow(clippy::unused_io_amount)]
        stream.read(&mut buffer).await?;

        // Send session accept
        let accept_bytes = hex_decode(SESSION_ACCEPT, &Endianness::Little).unwrap();
        stream.write_all(&accept_bytes).await?;

        // Read server response
        #[allow(clippy::unused_io_amount)]
        stream.read(&mut buffer).await?;
        stream.shutdown().await?;

        Self::parse_response(&buffer).await
    }

    async fn parse_response(buffer: &[u8; BUFFER_SIZE]) -> Result<Self, PatchInfoError> {
        let mut cursor: Cursor<[u8; BUFFER_SIZE]> = Cursor::new(*buffer);

        if !cursor.verify_food_header().await? {
            return Err(PatchInfoError::InvalidHeaderSequence);
        }

        // Skip unused fields
        let _content_length = cursor.read_u16_le().await?;
        let _ = cursor.read_u32_le().await?;
        /*
            isControl u8
            opCode u8
            Padding u16
        */

        // Verify protocol headers
        let service_id = cursor.read_u8().await?;
        let message_id = cursor.read_u8().await?;

        if service_id != SERVICE_ID || message_id != MESSAGE_ID {
            return Err(PatchInfoError::InvalidProtocol(service_id, message_id));
        }

        let _dml_length = cursor.read_u16_le().await?;
        let _latest_version = cursor.read_u32_le().await?;
        let _list_file_name = cursor.read_bytestring().await?;
        let _ = cursor.read_u128_le().await?;
        /*
            ListFileType u32
            ListFileTime u32
            ListFileSize u32
            ListFileCRC  u32
        */
        let list_file_url = cursor.read_bytestring().await?;
        let url_prefix = cursor.read_bytestring().await?;

        Ok(PatchInfo {
            list_file_url: list_file_url.clone(),
            url_prefix,
            revision: Self::parse_revision(&list_file_url)?,
        })
    }

    fn parse_revision(url: &str) -> Result<String, PatchInfoError> {
        let reg = Regex::new(r"/(V_[^/]+)/")?;

        if let Some(cap) = reg.captures(url).and_then(|c| c.get(1)) {
            return Ok(cap.as_str().to_string());
        }

        Err(PatchInfoError::InvalidRevisionFormat)
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
        let url = "http://versionak.us.wizard101.com/WizPatcher/V_r774907.Wizard_1_570/Windows/LatestFileList.bin";
        let result = PatchInfo::parse_revision(url).unwrap();

        assert_eq!(result, "V_r774907.Wizard_1_570");
    }
}
