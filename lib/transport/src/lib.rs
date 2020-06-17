//! transport module
//!
//! ### Protocol definition
//! |-------------------------------------------------------------------------------------------------|
//! |                    header               |                            body                       |
//! |-----------------|---------|-------------|-------------|-------------|---------|--------|--------|
//! |  fixed header   |  flag   |  body len   |  name size  |  timestamp  |  codec  |  name  |  data  |
//! |-----------------------------------------|-------------|-------------|---------|--------|--------|
//! |  4byte          |  1byte  |  4byte      |  1byte      |  4byte      |  1byte  |  x     |  x     |
//! |  0x99999909     |  x      |  x          |  x          |  x          |  x      |  x     |  x     |
//! |-------------------------------------------------------------------------------------------------|
//!
//! * `flag` Flag bit, user defined.
//! * `body len` Packet length.
//!

use std::error::Error;
use std::mem::transmute;
use bytes::{Buf, BufMut, BytesMut};

/// Message type
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Flag {
    Unknown = 0,
    Video = 1,  // 视频数据
    Audio = 2,  // 音频数据
    Frame = 3,  // 媒体规格数据
    Publish = 4,  // 推送事件
    UnPublish = 5,  // 停止推送事件
    Pull = 6,  // 获取事件
    Control = 7,  // 控制信息
}

// Events
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Event {
    Unknown = 0,
    Avg = 1,  // 负载数据
    Register = 2,  // 注册事件
    Flv = 3,
}

/// Data payload
#[derive(Clone, Debug)]
pub struct Payload {
    pub timestamp: u32,
    pub event: Event,
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
        packet.put_u8(payload.event as u8);

        // 写入频道名
        // 写入音视频数据
        packet.put(payload.name.as_bytes());
        packet.extend_from_slice(&payload.data);

        packet
    }

    /// 解包打包完成的RTMP数据
    ///
    /// 解包出RTMP数据和频道名和时间戳.
    pub fn parse(mut buffer: BytesMut) -> Result<Payload, Box<dyn Error>> {
        let size = buffer.get_u8();
        let timestamp = buffer.get_u32();
        let event = unsafe { transmute::<u8, Event>(buffer.get_u8()) };
        let data = buffer.split_off(size as usize);
        let name = String::from_utf8(buffer.to_vec())?;
        Ok(Payload { timestamp, event, name, data })
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

    /// 推入新的分片
    /// 
    /// 将接收到的缓冲区推入到内部缓冲区，
    /// 等待下次解码并消费掉;
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// let mut transport = Transport::new();
    /// 
    /// transport.push(transport.encoder(Bytes::from("hello"), 1).freeze());
    /// transport.push(transport.encoder(Bytes::from("world"), 2).freeze());
    /// transport.decoder()
    /// ```
    pub fn push(&mut self, chunk: BytesMut) {
        self.buffer.extend_from_slice(&chunk);
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
    /// transport.push(buffer.freeze());
    /// transport.decoder()
    /// ```
    #[rustfmt::skip]
    #[allow(dead_code)]
    pub fn decoder(&mut self) -> Option<Vec<(Flag, BytesMut)>> {
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
            // Get body length
            let flag = unsafe { transmute::<u8, Flag>(self.buffer[4]) };
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
