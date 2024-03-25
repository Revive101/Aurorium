use crate::{
    errors::RevisionError,
    util::{hex_decode, Endianness},
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use regex::Regex;
use std::{
    io::{Cursor, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};
use tokio::io::AsyncReadExt;

const BUFFER_SIZE: usize = 256;

// This just looks smart 🤓
#[async_trait]
pub trait WizIntegration {
    async fn read_bytestring(&mut self) -> String;
    async fn is_food_header(&mut self) -> bool;
}

#[async_trait]
impl WizIntegration for Cursor<[u8; BUFFER_SIZE]> {
    async fn read_bytestring(&mut self) -> String {
        let len = self.read_u16_le().await.unwrap();
        let mut buff = vec![0u8; len as usize];

        tokio::io::AsyncReadExt::read_exact(self, &mut buff)
            .await
            .unwrap();

        String::from_utf8_lossy(&buff).to_string()
    }

    async fn is_food_header(&mut self) -> bool {
        let food_header = self.read_u16_le().await.unwrap();
        food_header.to_le_bytes().eq(&FOOD_HEADER)
    }
}

const URL: &str = "patch.us.wizard101.coms";
const PORT: &str = "12500";
const FOOD_HEADER: [u8; 2] = [0x0D, 0xF0];
const SESSION_ACCEPT: &str =
    "0DF02700000000000802220000000000000000000000000000000000000000000000000000000000000000";
const SERVICE_ID: u8 = 8; // PATCH
const MESSAGE_ID: u8 = 2; // MSG_LATEST_FILE_LIST_V2

pub struct Revision {
    pub list_file_url: String,
    pub url_prefix: String,
    pub revision: String,
}
impl Revision {
    fn create_stream() -> std::io::Result<TcpStream> {
        let mut ip = format!("{URL}:{PORT}").to_socket_addrs()?;
        let stream = TcpStream::connect_timeout(&ip.next().unwrap(), Duration::from_secs(20));
        log::info!("Successfully connected to {URL}");

        stream
    }

    pub async fn check() -> Result<Revision> {
        let mut stream = Self::create_stream().with_context(|| format!("{URL}:{PORT}"))?;

        let mut buffer = [0u8; BUFFER_SIZE];
        stream.read(&mut buffer)?; // We don't need the SessionOffer
        buffer = [0u8; BUFFER_SIZE];

        stream.write_all(&hex_decode(SESSION_ACCEPT, &Endianness::Little).unwrap()[..])?;

        stream.read(&mut buffer)?;
        let mut cursor: Cursor<[u8; BUFFER_SIZE]> = Cursor::new(buffer);

        if !cursor.is_food_header().await {
            log::error!("Received invalid Header sequence");
            return Err(anyhow!(RevisionError::InvalidHeaderSequence));
        }

        let _ = cursor.read_u16_le().await?;
        let _ = cursor.read_u32_le().await?;

        let service_id = cursor.read_u8().await?;
        let message_id = cursor.read_u8().await?;

        if service_id != SERVICE_ID || message_id != MESSAGE_ID {
            log::error!("Expected SERVICE_ID=8 & MESSAGE_ID=2 but got {service_id} & {message_id}");
            return Err(anyhow!(RevisionError::InvalidProtocol(
                service_id, message_id
            )));
        }

        let _dml_length = cursor.read_u16_le().await?;
        let _latest_version = cursor.read_u32_le().await?;
        let _list_file_name = cursor.read_bytestring().await;
        let _ = cursor.read_u128_le().await?;
        let list_file_url = cursor.read_bytestring().await;
        let url_prefix = cursor.read_bytestring().await;

        stream.shutdown(std::net::Shutdown::Both)?;

        Ok(Revision {
            list_file_url: list_file_url.clone(),
            url_prefix,
            revision: Self::parse_revision(&list_file_url),
        })
    }

    pub fn parse_revision(url: &str) -> String {
        let reg = Regex::new(r"/(V_[^/]+)/").unwrap();

        if let Some(captures) = reg.captures(url) {
            if let Some(version) = captures.get(1) {
                return version.as_str().to_string();
            }
        }

        String::new()
    }
}
