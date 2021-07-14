mod payload;

use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use payload::Payload;
use bytes::Buf;

#[repr(u8)]
#[derive(TryFromPrimitive)]
pub enum Content {
    Handshake = 0x16
}

pub struct Dtls {
    content: Content,
    version: u16,
    epoch: u16,
    sequence: u64,
    length: u16,
    fragment: Vec<Payload>
}

impl TryFrom<&[u8]> for Dtls {
    type Error = anyhow::Error;
    fn try_from(mut buf: &[u8]) -> Result<Self, Self::Error> {
        let content = Content::try_from(buf.get_u8())?;
        let version = buf.get_u16();
        let epoch = buf.get_u16();
        let sequence = buf.get_uint(6);
        let length = buf.get_u16();

        Ok(Self {
            content,
            version,
            epoch,
            length,
            sequence,
            fragment: Vec::with_capacity(5)
        })
    }
}
