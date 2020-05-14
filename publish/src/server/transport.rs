//! UDP transport module
//!
//! ### Protocol definition
//!
//! |  name  |  flag  |  message id  |  len   |  package id  |  package is end  |  data  |
//! |--------|--------|--------------|--------|--------------|------------------|--------|
//! |  len   |  1byte |  4byte       |  4byte |  1byte       |  1byte           |  x     |
//! |  data  |  x     |  x           |  x     |  x           |  0 | 1           |  x     |
//!
//! * `flag` Flag bit, user defined.
//! * `message id` Message ID, the serial number of the current message.
//! * `len` Packet length.
//! * `package id` Package ID, the serial number of the current package.
//! * `package is end` Whether the current packet is over, not over 0, over 1.
//!
//! TODO:
//! 消息序号最大为u32, 对端应该自动溢出归0;

use bytes::{BufMut, Bytes, BytesMut};

/// UDP transport module.
///
/// Processing audio and video data, packaging audio and
/// video data into Udp packets, and automatically processing 
/// the maximum transmission unit limit.
pub struct Transport {
    max_index: u32,
    index: u32,
    mtu: u32,
}

impl Transport {
    /// Create a transport instance
    ///
    /// The maximum transmission unit size should be specified when initializing the instance.
    /// Note: The maximum transmission unit size does not represent the final 
    /// size of the data packet. The module will write some control information and 
    /// sequence number to attach to the data packet, so pay attention to leaving 
    /// about 12 bytes of redundancy here.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use transport::Transport;
    ///
    /// Transport::new(1000);
    /// ```
    pub fn new(mtu: u32) -> Self {
        Self {
            mtu,
            index: 0,
            max_index: u32::max_value(),
        }
    }

    /// Create Udp packet
    ///
    /// According to MTU, it is automatically divided into multiple 
    /// Udp data packets, and the packet sequence number and control 
    /// information are marked (whether it is over).
    ///
    /// TODO: 未解决问题，对端会收到空包，但是长度为8;
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use transport::Transport;
    ///
    /// let mut transport = Transport::new(1000);
    /// transport.packet(b"hello");
    /// ```
    pub fn packet(&mut self, chunk: Bytes, flgs: u8) -> Vec<Bytes> {
        // MTU size
        // Packet size
        let size = self.mtu as usize;
        let sum_size = chunk.len();

        // Udp package list
        // Offset to write data
        // Package serial number
        let mut packets = Vec::new();
        let mut offset: usize = 0;
        let mut index: u8 = 0;

        // Infinite loop
        // Until the allocation is completed
        loop {
            let mut package = BytesMut::new();

            // To avoid overflowing access to pointers
            // If it exceeds the range, only the maximum range is specified
            let end = if offset + size > sum_size {
                sum_size
            } else {
                offset + size
            };

            // write mark bit
            // Write message sequence number
            // Write packet sequence number
            package.put_u8(flgs);
            package.put_u32(self.index);
            package.put_u32((end - offset) as u32);
            package.put_u8(index);

            // Check if the package is over
            // If finished, write end bit
            if sum_size == end {
                package.put_u8(1u8);
            } else {
                package.put_u8(0u8);
            }

            // Write data in the data packet range
            // Add the packet to the list
            package.extend_from_slice(&chunk[offset..end]);
            packets.push(package.freeze());

            // If the writing has been completed, 
            // the loop will jump out, If the writing 
            // is not completed, adjust the offset and 
            // serial number.
            if sum_size == end {
                break;
            } else {
                index += 1;
                offset = end;
            }
        }

        // Check if the maximum value of U32 is exceeded
        // If the maximum value is exceeded, then return to zero.
        if self.index + 1 > self.max_index {
            self.index = 0;
        } else {
            self.index += 1;
        }

        packets
    }
}
