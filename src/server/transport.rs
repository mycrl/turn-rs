use bytes::{Bytes, BytesMut, BufMut};

pub struct Transport {
    index: u32,
    mtu: u32
}

impl Transport {
    pub fn new(mtu: u32) -> Self {
        Self {
            mtu,
            index: 0
        }
    }

    pub fn packet (&mut self, chunk: Bytes, flgs: u8) -> Vec<Bytes> {
        let mut package = BytesMut::new();
        package.put_u8(flgs);
        package.put_u32(self.index);
        package.put_u8(index);
        package.extend_from_slice(&chunk);
        self.index += 1;

        match package.len() > self.mtu as usize {
            true => self.package(package),
            false => vec![ Bytes::from(package) ]
        }
    }
}
