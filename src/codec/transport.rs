use bytes::Bytes;
use byteorder::{WriteBytesExt, BigEndian};
use std::io::Error;

pub struct Transport {
    index: u32
}

impl Transport {
    pub fn new() -> Self {
        Self {
            index: 0
        }
    }

    pub fn packet (&mut self, chunk: Bytes, flgs: u8) -> Result<Bytes, Error> {
        let mut package = Vec::new();
        package.write_u8(flgs)?;
        package.write_u32::<BigEndian>(self.index)?;
        package.extend_from_slice(&chunk);

        self.index += 1;
        
        Ok(Bytes::from(package))
    }
}