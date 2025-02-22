#![allow(unused)]

use crate::{
    errors::RevisionError,
    util::{hex_decode, Endianness},
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use std::{io::Cursor, net::ToSocketAddrs};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const BUFFER_SIZE: usize = 256;

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

        tokio::io::AsyncReadExt::read_exact(self, &mut buff).await.unwrap();

        String::from_utf8_lossy(&buff).to_string()
    }

    async fn is_food_header(&mut self) -> bool {
        let food_header = self.read_u16_le().await.unwrap();
        food_header.to_le_bytes().eq(&FOOD_HEADER)
    }
}

const URL: &str = "patch.us.wizard101.com";
const PORT: &str = "12500";
const FOOD_HEADER: [u8; 2] = [0x0D, 0xF0];
const SESSION_ACCEPT: &str = "0DF02700000000000802220000000000000000000000000000000000000000000000000000000000000000";
const SERVICE_ID: u8 = 8; // PATCH
const MESSAGE_ID: u8 = 2; // MSG_LATEST_FILE_LIST_V2

#[derive(Clone)]
pub struct Revision {
    pub list_file_url: String,
    pub url_prefix: String,
    pub revision: String,
}
impl Revision {
    async fn create_stream() -> std::io::Result<TcpStream> {
        let mut ip = format!("{URL}:{PORT}").to_socket_addrs()?;

        TcpStream::connect(&ip.next().unwrap()).await
    }

    pub async fn check_for_new_revision() -> Result<Revision> {
        let mut stream = Self::create_stream().await?;
        println!("Successfully connected to {URL}");

        let mut buffer = [0u8; BUFFER_SIZE];
        stream.read(&mut buffer).await?; // We don't need the SessionOffer
        buffer = [0u8; BUFFER_SIZE];

        stream
            .write_all(&hex_decode(SESSION_ACCEPT, &Endianness::Little).unwrap()[..])
            .await?;

        stream.read(&mut buffer).await?;
        stream.shutdown().await?;

        let mut cursor: Cursor<[u8; BUFFER_SIZE]> = Cursor::new(buffer);

        if !cursor.is_food_header().await {
            eprintln!("Received invalid Header sequence");
            return Err(anyhow!(RevisionError::InvalidHeaderSequence));
        }

        let _ = cursor.read_u16_le().await?;
        let _ = cursor.read_u32_le().await?;

        let service_id = cursor.read_u8().await?;
        let message_id = cursor.read_u8().await?;

        if service_id != SERVICE_ID || message_id != MESSAGE_ID {
            return Err(anyhow!(RevisionError::InvalidProtocol(service_id, message_id)));
        }

        let _dml_length = cursor.read_u16_le().await?;
        let _latest_version = cursor.read_u32_le().await?;
        let _list_file_name = cursor.read_bytestring().await;
        let _ = cursor.read_u128_le().await?;
        let list_file_url = cursor.read_bytestring().await;
        let url_prefix = cursor.read_bytestring().await;

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
