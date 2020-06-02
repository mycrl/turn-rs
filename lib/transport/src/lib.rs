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

/// message type
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Flag {
    Video = 0,
    Audio = 1,
    Frame = 2,
    Publish = 3,
    UnPublish = 4,
    Pull = 5,
    Avg = 6,
    None,
}

/// 数据包定义
#[derive(Clone, Debug)]
pub struct Payload {
    pub timestamp: u32,
    pub name: String,
    pub data: BytesMut,
}

/// Transport Protocol Codec
///
/// Implementation of internal Tcp transfer protocol,
/// Including encoder and decoder.
#[allow(dead_code)]
#[derive(Default)]
pub struct Transport {
    buffer: BytesMut,
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
            buffer: BytesMut::new(),
        }
    }

    /// 打包RTMP数据
    ///
    /// 打包RTMP数据，并包含数据的频道和时间戳信息.
    /// 注意：如果没有时间戳，应该为0.
    pub fn packet(payload: Payload) -> BytesMut {
        let mut packet = BytesMut::new();
        let size = payload.name.len() as u8;

        // 写入频道名长度
        // 写入时间戳
        packet.put_u8(size);
        packet.put_u32(payload.timestamp);

        // 写入频道名
        // 写入音视频数据
        packet.put(payload.name.as_bytes());
        packet.extend_from_slice(&payload.data);

        packet
    }

    /// 解包打包完成的RTMP数据
    ///
    /// 解包出RTMP数据和频道名和时间戳.
    pub fn parse(mut buffer: BytesMut) -> Result<Payload, Box<dyn std::error::Error>> {
        let size = buffer.get_u8();
        let timestamp = buffer.get_u32();
        let data = buffer.split_off(size as usize);
        let name = String::from_utf8(buffer.to_vec())?;
        Ok(Payload {
            timestamp,
            name,
            data,
        })
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
    pub fn encoder(chunk: BytesMut, flag: Flag) -> BytesMut {
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
        packet.put_u8(flag as u8);
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
    /// the next time.
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
    pub fn decoder(&mut self, chunk: BytesMut) -> Option<Vec<(Flag, BytesMut)>> {
        self.buffer.extend_from_slice(&chunk);
        let mut receiver = Vec::new();

        loop {

            // Check whether the remaining data is sufficient 
            // to complete the next analysis, otherwise please 
            // do not waste time.
            if self.buffer.len() < 9 {
                break;
            }

            // Get the fixed head
            let head = u32::from_be_bytes([
                self.buffer[0],
                self.buffer[1],
                self.buffer[2],
                self.buffer[3]
            ]);

            // Check the fixed head
            if head != 0x99999909 {
                break;
            }

            // Get the flag
            let flag = match self.buffer[4] {
                0 => Flag::Video,
                1 => Flag::Audio,
                2 => Flag::Frame,
                3 => Flag::Publish,
                4 => Flag::UnPublish,
                5 => Flag::Pull,
                _ => Flag::None
            };

            // Get body length
            let size = u32::from_be_bytes([
                self.buffer[5],
                self.buffer[6],
                self.buffer[7],
                self.buffer[8]
            ]) as usize;

            // Check if the body meets the length
            let sum_size = size + 9;
            if sum_size > self.buffer.len() {
                break;
            }

            // Get data frame
            let body = BytesMut::from(&self.buffer[9..sum_size]);
            receiver.push((flag, body));
            self.buffer.advance(sum_size);
        }

        match &receiver.is_empty() {
            false => Some(receiver),
            true => None
        }
    }
}
