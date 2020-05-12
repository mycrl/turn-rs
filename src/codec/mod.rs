pub mod rtmp;

use bytes::Bytes;

pub enum Packet {
    Tcp(Bytes),
    Udp(Bytes, u8)
}

pub trait Codec {
    fn parse(&mut self, buffer: Bytes) -> Vec<Packet>;
}
