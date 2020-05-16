//! transport module
//!
//! ### Protocol definition
//!
//! |  name  |  fixed header   |  flag   |  body len   |  body  |
//! |--------|-----------------|---------|-------------|--------|
//! |  len   |  4byte          |  1byte  |  4byte      |  x     |
//! |  data  |  0x99999909     |  x      |  x          |  x     |
//!
//! * `flag` Flag bit, user defined.
//! * `body len` Packet length.
//!
//! 

use bytes::{Buf, BufMut, BytesMut};

/// Transport Protocol Codec
///
/// Implementation of internal Tcp transfer protocol,
/// Including encoder and decoder.
#[allow(dead_code)]
pub struct Transport {
    buffer: BytesMut
}

impl Transport {
    /// Create a transshipment agreement example
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// let mut transport = Transport::new();
    /// let mut buffer = BytesMut::new();
    /// 
    /// buffer.put(transport.encode(BytesMut::from("hello"), 1));
    /// buffer.put(transport.encode(BytesMut::from("world"), 2));
    /// 
    /// let result = transport.decode(buffer.freeze());
    /// assert_eq!(result, Some(vec![
    ///     (1, BytesMut::from("hello")),
    ///     (2, BytesMut::from("world"))
    /// ]));
    /// ```
    pub fn new() -> Self {
        Self { 
            buffer: BytesMut::new()
        }
    }

    /// Encode data into protocol frames
    /// 
    /// The user can define the flag bit to indicate the 
    /// type of the current data frame. It does not consider 
    /// too many complicated implementations, but simply 
    /// distinguishes the packet type. The actual specific 
    /// data requires the user to decode the data frame.
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// let mut transport = Transport::new();
    /// let mut buffer = BytesMut::new();
    /// 
    /// transport.encoder(Bytes::from("hello"), 1)
    /// ```
    #[rustfmt::skip]
    #[allow(dead_code)]
    pub fn encoder(&mut self, chunk: BytesMut, flag: u8) -> BytesMut {
        let mut packet = BytesMut::new();
        let size = chunk.len() as u32;

        // Write fixed head
        packet.extend_from_slice(&[

            // head
            0x99, 
            0x99,
            0x99,

            // fixed header size
            0x09
        ]);

        // Write flag
        // Write body length
        packet.put_u8(flag);
        packet.put_u32(size);

        // Write body
        packet.extend_from_slice(&chunk);

        // Returns bytes
        packet
    }

    /// Decode protocol frames
    /// 
    /// Write data shards, try to decode all data shards, 
    /// maintain a buffer internally, and automatically 
    /// accumulate data that has not been processed for 
    /// the next time
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// let mut transport = Transport::new();
    /// let mut buffer = BytesMut::new();
    /// 
    /// buffer.put(transport.encoder(Bytes::from("hello"), 1));
    /// buffer.put(transport.encoder(Bytes::from("world"), 2));
    /// 
    /// transport.decoder(buffer.freeze())
    /// ```
    #[rustfmt::skip]
    #[allow(dead_code)]
    pub fn decoder(&mut self, chunk: BytesMut) -> Option<Vec<(u8, BytesMut)>> {
        self.buffer.extend_from_slice(&chunk);
        let mut receiver = Vec::new();

        loop {

            // Check the fixed head
            let head = self.buffer.get_u32();
            if head != 0x99999909 {
                break;
            }

            // Get the flag
            // Get body length
            // Check if the body meets the length
            let flag = self.buffer.get_u8();
            let size = self.buffer.get_u32() as usize;
            let last = self.buffer.remaining();
            if last < size {
                break;
            }

            // Get data frame
            let body = BytesMut::from(&self.buffer[0..size]);
            receiver.push((flag, body));
            self.buffer.advance(size);

            // Check whether the remaining data is sufficient 
            // to complete the next analysis, otherwise please 
            // do not waste time.
            if self.buffer.len() < 10 {
                break;
            }
        }

        match &receiver.is_empty() {
            false => Some(receiver),
            true => None
        }
    }
}
