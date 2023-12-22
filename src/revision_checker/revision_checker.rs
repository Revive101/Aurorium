use async_trait::async_trait;
use regex::Regex;
use std::{
    io::{Cursor, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};
use tokio::io::AsyncReadExt;

use crate::{
    errors::RevisionError,
    util::{hex_decode, Endianness},
};

const URL: &str = "patch.us.wizard101.com";
const PORT: &str = "12500";
const MAGIC_HEADER: [u8; 2] = [0x0D, 0xF0];
const SESSION_ACCEPT: &str =
    "0DF02700000000000802220000000000000000000000000000000000000000000000000000000000000000";
const SERVICE_ID: u8 = 8; // PATCH
const MESSAGE_ID: u8 = 2; // MSG_LATEST_FILE_LIST_V2

type ByteCursor<const N: usize> = Cursor<[u8; N]>;
#[async_trait]
pub trait WizIntegration {
    async fn read_bytestring<const N: usize>(&mut self) -> String;
    async fn is_magic_header<const N: usize>(&mut self) -> bool;
}

#[async_trait]
impl<const M: usize> WizIntegration for ByteCursor<M> {
    async fn read_bytestring<const N: usize>(&mut self) -> String {
        let len = self.read_u16_le().await.unwrap();
        let mut buff = vec![0u8; len as usize];

        tokio::io::AsyncReadExt::read_exact(self, &mut buff)
            .await
            .unwrap();

        String::from_utf8_lossy(&buff).to_string()
    }

    async fn is_magic_header<const N: usize>(&mut self) -> bool {
        let magic_header = self.read_u16_le().await.unwrap();
        magic_header.to_le_bytes().eq(&MAGIC_HEADER)
    }
}

pub struct Revision {
    pub list_file_url: String,
    pub url_prefix: String,
    pub revision: String,
}
impl Revision {
    fn create_stream() -> std::io::Result<TcpStream> {
        let mut ip = format!("{URL}:{PORT}").to_socket_addrs()?;
        log::info!("Successfully connected to {URL}");

        TcpStream::connect_timeout(&ip.next().unwrap(), Duration::from_secs(20))
    }

    pub async fn check<const N: usize>() -> Result<Revision, RevisionError> {
        let mut stream = Self::create_stream()?;

        let mut buffer = [0u8; N];
        stream.read(&mut buffer)?; // We don't need the SessionOffer
        buffer = [0u8; N];

        stream.write_all(&hex_decode(SESSION_ACCEPT, &Endianness::Little).unwrap()[..])?;

        stream.read(&mut buffer)?;
        let mut cursor: ByteCursor<N> = Cursor::new(buffer);

        if !Self::is_magic_header(&mut cursor).await {
            log::error!("Received invalid MagicHeader sequence");
            return Err(RevisionError::InvalidMagicHeader);
        }

        let _ = cursor.read_u16_le().await?;
        let _ = cursor.read_u32_le().await?;

        let service_id = cursor.read_u8().await?;
        let message_id = cursor.read_u8().await?;

        if service_id != SERVICE_ID || message_id != MESSAGE_ID {
            log::error!(
                "Expected SERVICE_ID (8) & MESSAGE_ID (2) but got {service_id} & {message_id}"
            );
            return Err(RevisionError::InvalidProtocol(service_id, message_id));
        }

        let _dml_length = cursor.read_u16_le().await?;
        let _latest_version = cursor.read_u32_le().await?;
        let _list_file_name = Self::read_bytestring(&mut cursor).await;
        let _ = cursor.read_u128_le().await?;
        let list_file_url = Self::read_bytestring(&mut cursor).await;
        let url_prefix = Self::read_bytestring(&mut cursor).await;

        stream.shutdown(std::net::Shutdown::Both)?;

        Ok(Revision {
            list_file_url: list_file_url.clone(),
            url_prefix,
            revision: Self::parse_revision(&list_file_url),
        })
    }

    pub fn parse_revision(url: &str) -> String {
        let reg = Regex::new(r"/V_([^/]+)/").unwrap();

        if let Some(captures) = reg.captures(url) {
            if let Some(version) = captures.get(1) {
                return format!("V_{}", version.as_str());
            }
        }

        String::new()
    }
}
