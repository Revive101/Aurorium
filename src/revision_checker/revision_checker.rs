use crate::util::{hex_decode, Endianness};
use std::{
    io::{Cursor, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const URL: &str = "patch.us.wizard101.com";
const PORT: &str = "12500";
const MAGIC_HEADER: [u8; 2] = [0x0D, 0xF0];

pub struct RevisionChecker {
    stream: TcpStream,
}
impl RevisionChecker {
    pub fn new() -> Result<Self, ()> {
        if let Ok(mut ip) = format!("{URL}:{PORT}").to_socket_addrs() {
            log::info!("Successfully connected to {URL}");
            Ok(Self {
                stream: TcpStream::connect_timeout(&ip.next().unwrap(), Duration::from_secs(20))
                    .unwrap(),
            })
        } else {
            log::error!("Unable to reach host '{URL}'! (Aborting)");
            Err(())
        }
    }

    pub async fn start<const N: usize>(mut self) {
        let input_str = "0DF02700000000000802220000000000000000000000000000000000000000000000000000000000000000";
        self.stream
            .write_all(&hex_decode(input_str, Endianness::Little).unwrap()[..])
            .unwrap();

        let mut buffer = [0u8; N];
        loop {
            match self.stream.read(&mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        log::info!("Server disconnected");
                        break;
                    }

                    let mut cursor = Cursor::new(buffer);
                    if !Self::is_magic_header(&mut cursor).await {
                        log::error!("Received invalid MagicHeader sequence (Aborting)");
                        break;
                    }

                    let length = cursor.read_u16_le().await.unwrap();
                    let big_length = cursor.read_u32_le().await.unwrap();
                    println!("Length {length} ;  BigLength {big_length}");

                    let session_id = cursor.read_u16_le().await.unwrap();
                    println!("SessionID {session_id}");

                    let mut send_buff = Cursor::new(Vec::new());
                    tokio::io::AsyncWriteExt::write_all(
                        &mut send_buff,
                        &hex_decode("0DF015000105000000000000000000", Endianness::Little).unwrap()
                            [..],
                    )
                    .await
                    .unwrap();
                    send_buff.write_u16_le(session_id).await.unwrap();
                    send_buff.write_u8(0).await.unwrap();

                    println!("{:02X?}", buffer);
                    println!("{}", String::from_utf8_lossy(&buffer).to_string());
                    self.stream.write_all(&send_buff.into_inner()[..]).unwrap();
                }
                Err(why) => {
                    panic!("{why}");
                }
            }

            buffer = [0u8; N];
        }
    }

    async fn is_magic_header<const N: usize>(cursor: &mut Cursor<[u8; N]>) -> bool {
        let magic_header = cursor.read_u16_le().await.unwrap();
        magic_header.to_le_bytes().eq(&MAGIC_HEADER)
    }
}
