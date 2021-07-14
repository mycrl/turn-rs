use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

#[repr(u8)]
#[derive(TryFromPrimitive)]
pub enum Kind {
    ClientHello = 0x01
}

pub struct Handshake {
    kind: Kind,
    length: u32,
    sequence: u16,
    offset: u32,
    version: u16,
    fragment_length: u16,
}

impl TryFrom<&[u8]> for Handshake {
    type Error = anyhow::Error;
    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        
    }
}
