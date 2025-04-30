use crate::{
    errors::RevisionError,
    util::{Endianness, hex_decode},
};
use regex::Regex;
use std::{io::Cursor, net::ToSocketAddrs};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub mod checker;
pub mod fetcher;
pub mod xml_parser;

const BUFFER_SIZE: usize = 256;
const FOOD_HEADER: [u8; 2] = [0x0D, 0xF0];
const URL: &str = "patch.us.wizard101.com"; //? Arg
const PORT: &str = "12500"; //? Arg
const SESSION_ACCEPT: &str = "0DF02700000000000802220000000000000000000000000000000000000000000000000000000000000000";
const SERVICE_ID: u8 = 8; // PATCH
const MESSAGE_ID: u8 = 2; // MSG_LATEST_FILE_LIST_V2

pub trait WizIntegration {
    fn read_bytestring(&mut self) -> impl Future<Output = Result<String, RevisionError>> + Send;
    fn verify_food_header(&mut self) -> impl Future<Output = Result<bool, RevisionError>> + Send;
}

impl WizIntegration for Cursor<[u8; BUFFER_SIZE]> {
    async fn read_bytestring(&mut self) -> Result<String, RevisionError> {
        let len = self.read_u16_le().await?;
        let mut buffer = vec![0u8; len as usize];
        tokio::io::AsyncReadExt::read_exact(self, &mut buffer).await?;
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    async fn verify_food_header(&mut self) -> Result<bool, RevisionError> {
        let food_header = self.read_u16_le().await?;
        Ok(food_header.to_le_bytes() == FOOD_HEADER)
    }
}

#[derive(Debug)]
pub struct Revision {
    pub list_file_url: String,
    pub url_prefix: String,
    pub revision: String,
}

impl Revision {
    pub async fn check() -> Result<Self, RevisionError> {
        let mut ip = format!("{URL}:{PORT}").to_socket_addrs()?;
        let mut stream = TcpStream::connect(&ip.next().unwrap()).await?;

        // Read and discard initial session offer
        let mut buffer = [0u8; BUFFER_SIZE];
        stream.read(&mut buffer).await?;
        buffer = [0u8; BUFFER_SIZE];

        // Send session accept
        let accept_bytes = hex_decode(SESSION_ACCEPT, &Endianness::Little).unwrap();
        stream.write_all(&accept_bytes).await?;

        // Read server response
        stream.read(&mut buffer).await?;
        stream.shutdown().await?;

        Self::parse_server_response(&buffer).await
    }

    async fn parse_server_response(buffer: &[u8; BUFFER_SIZE]) -> Result<Self, RevisionError> {
        let mut cursor: Cursor<[u8; BUFFER_SIZE]> = Cursor::new(*buffer);

        if !cursor.verify_food_header().await? {
            return Err(RevisionError::InvalidHeaderSequence);
        }

        // Skip unused fields
        let _ = cursor.read_u16_le().await?;
        let _ = cursor.read_u32_le().await?;

        // Verify protocol headers
        let service_id = cursor.read_u8().await?;
        let message_id = cursor.read_u8().await?;

        if service_id != SERVICE_ID || message_id != MESSAGE_ID {
            return Err(RevisionError::InvalidProtocol(service_id, message_id));
        }

        let _dml_length = cursor.read_u16_le().await?;
        let _latest_version = cursor.read_u32_le().await?;
        let _list_file_name = cursor.read_bytestring().await?;
        let _ = cursor.read_u128_le().await?;
        let list_file_url = cursor.read_bytestring().await?;
        let url_prefix = cursor.read_bytestring().await?;

        Ok(Revision {
            list_file_url: list_file_url.clone(),
            url_prefix,
            revision: Self::parse_revision(&list_file_url),
        })
    }

    fn parse_revision(url: &str) -> String {
        let reg = Regex::new(r"/(V_[^/]+)/").unwrap();

        if let Some(captures) = reg.captures(url) {
            if let Some(version) = captures.get(1) {
                return version.as_str().to_string();
            }
        }

        String::new()
    }
}
