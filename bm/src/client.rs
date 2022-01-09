use apiformes_packet::prelude::*;
use bytes::{Buf, BytesMut};
use std::io::Cursor;
use std::io::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};

pub struct Client {
    stream: TcpStream,
    recv_bytes: BytesMut,
    send_bytes: BytesMut,
}

impl Client {
    pub async fn new<A: ToSocketAddrs>(addr: A) -> Result<Client> {
        Ok(Client {
            stream: TcpStream::connect(addr).await?,
            send_bytes: BytesMut::with_capacity(128),
            recv_bytes: BytesMut::with_capacity(128),
        })
    }
    pub async fn recv(&mut self) -> Result<Packet> {
        loop {
            let mut cursor = Cursor::new(&self.recv_bytes[..]);
            match Packet::from_bytes(&mut cursor) {
                Ok(packet) => {
                    self.recv_bytes.advance(packet.frame_len());
                    return Ok(packet);
                }
                Err(DataParseError::InsufficientBuffer {
                    needed: _,
                    available: _,
                }) => self.stream.read_buf(&mut self.recv_bytes).await?,
                Err(e) => panic!("{:?}", e),
            };
        }
    }

    pub async fn send(&mut self, packet: &Packet) -> Result<()> {
        packet.to_bytes(&mut self.send_bytes);
        self.stream.write_all_buf(&mut self.send_bytes).await?;
        Ok(())
    }
}
