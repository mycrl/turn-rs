pub mod rtmp;
pub mod transport;

use bytes::Bytes;
use transport::Transport;

pub enum Packet {
    Tcp(Bytes),
    Udp(Bytes)
}

pub trait Codec {
    fn parse(&mut self, buffer: Bytes) -> Vec<Packet>;
}