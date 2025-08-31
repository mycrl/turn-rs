use std::ops::{Deref, DerefMut};

struct ExchangeBufferItem {
    buffer: Vec<u8>,
    len: usize,
}

impl Default for ExchangeBufferItem {
    fn default() -> Self {
        Self {
            buffer: vec![0u8; 4096],
            len: 0,
        }
    }
}

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
#[derive(Default)]
pub struct ExchangeBuffer {
    items: [ExchangeBufferItem; 2],
    index: usize,
}

impl Deref for ExchangeBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.items[self.index].buffer[..]
    }
}

impl DerefMut for ExchangeBuffer {
    // Writes need to take into account overwriting written data, so fetching the
    // writable buffer starts with the internal cursor.
    fn deref_mut(&mut self) -> &mut Self::Target {
        let len = self.items[self.index].len;
        &mut self.items[self.index].buffer[len..]
    }
}

impl ExchangeBuffer {
    pub fn len(&self) -> usize {
        self.items[self.index].len
    }

    /// The buffer does not automatically advance the cursor as BytesMut
    /// does, and you need to manually advance the length of the data
    /// written.
    pub fn advance(&mut self, len: usize) {
        self.items[self.index].len += len;
    }

    pub fn split(&mut self, len: usize) -> &[u8] {
        let current_len = self.items[self.index].len;
        let current_index = self.index;

        // The length of the separation cannot be greater than the length of the data.
        assert!(len <= current_len);

        // Length of unconsumed data
        let remaining = current_len - len;

        {
            // The current buffer is no longer in use, resetting the content length.
            self.items[self.index].len = 0;

            // Invert the buffer.
            self.index = if self.index == 0 { 1 } else { 0 };

            // The length of unconsumed data needs to be updated into the reversed
            // completion buffer.
            self.items[self.index].len = remaining;
        }

        // Unconsumed data exists and is copied to the free buffer.
        if remaining > 0 {
            // Use split_at_mut to get mutable references to both buffers
            let (left, right) = self.items.split_at_mut(1);
            
            // Determine which slice contains the source and target buffers
            let (source_slice, target_slice) = if current_index == 0 {
                (&left[0].buffer[len..current_len], &mut right[0].buffer[..remaining])
            } else {
                (&right[0].buffer[len..current_len], &mut left[0].buffer[..remaining])
            };
            
            target_slice.copy_from_slice(source_slice);
        }

        &self.items[current_index].buffer[..len]
    }
}
