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
        self.index += 1;

        let size = self.mtu as usize;
        if chunk.len() <= size {
            return vec![chunk]
        }

        let mut package = BytesMut::new();
        let mut offset = 0;
        // loop {
        //     package.put_u8(flgs);
        //     package.put_u32(self.index);
        //     package.put_u8(index);
        //     package.extend_from_slice(&chunk);
        // }

        vec![chunk]
    }
}
