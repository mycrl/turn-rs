use std::ops::{Deref, DerefMut};

/// An emulated double buffer queue, this is used when reading data over
/// TCP.
///
/// When reading data over TCP, you need to keep adding to the buffer until
/// you find the delimited position. But this double buffer queue solves
/// this problem well, in the queue, the separation is treated as the first
/// read operation and after the separation the buffer is reversed and
/// another free buffer is used for writing the data.
///
/// If the current buffer in the separation after the existence of
/// unconsumed data, this time the unconsumed data will be copied to another
/// free buffer, and fill the length of the free buffer data, this time to
/// write data again when you can continue to fill to the end of the
/// unconsumed data.
///
/// This queue only needs to copy the unconsumed data without duplicating
/// the memory allocation, which will reduce a lot of overhead.
pub struct ExchangeBuffer {
    buffers: [(Vec<u8>, usize /* len */); 2],
    index: usize,
}

impl Default for ExchangeBuffer {
    #[rustfmt::skip]
    fn default() -> Self {
            Self {
                index: 0,
                buffers: [
                    (vec![0u8; 4096], 0),
                    (vec![0u8; 4096], 0),
                ],
            }
        }
}

impl Deref for ExchangeBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buffers[self.index].0[..]
    }
}

impl DerefMut for ExchangeBuffer {
    // Writes need to take into account overwriting written data, so fetching the
    // writable buffer starts with the internal cursor.
    fn deref_mut(&mut self) -> &mut Self::Target {
        let len = self.buffers[self.index].1;
        &mut self.buffers[self.index].0[len..]
    }
}

impl ExchangeBuffer {
    pub fn len(&self) -> usize {
        self.buffers[self.index].1
    }

    /// The buffer does not automatically advance the cursor as BytesMut
    /// does, and you need to manually advance the length of the data
    /// written.
    pub fn advance(&mut self, len: usize) {
        self.buffers[self.index].1 += len;
    }

    pub fn split(&mut self, len: usize) -> &[u8] {
        let (ref current_bytes, current_len) = self.buffers[self.index];

        // The length of the separation cannot be greater than the length of the data.
        assert!(len <= current_len);

        // Length of unconsumed data
        let remaining = current_len - len;

        {
            // The current buffer is no longer in use, resetting the content length.
            self.buffers[self.index].1 = 0;

            // Invert the buffer.
            self.index = if self.index == 0 { 1 } else { 0 };

            // The length of unconsumed data needs to be updated into the reversed
            // completion buffer.
            self.buffers[self.index].1 = remaining;
        }

        // Unconsumed data exists and is copied to the free buffer.
        #[allow(mutable_transmutes)]
        if remaining > 0 {
            unsafe {
                std::mem::transmute::<&[u8], &mut [u8]>(&self.buffers[self.index].0[..remaining])
            }
            .copy_from_slice(&current_bytes[len..current_len]);
        }

        &current_bytes[..len]
    }
}
