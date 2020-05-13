pub mod rtmp;

use bytes::{BytesMut, Bytes};

/// 编解码器返回的数据包
pub enum Packet {
    /// Tcp消息
    Tcp(Bytes),

    /// Udp消息，包含标志位
    Udp(Bytes, u8)
}

pub trait Codec {
    fn parse(&mut self, buffer: &mut BytesMut) -> Vec<Packet>;
}
