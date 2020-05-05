use super::State;
use super::State::Callback;
use bytes::{Bytes, BytesMut, BufMut};
use rml_rtmp::chunk_io::{ChunkDeserializer, ChunkSerializer};
use rml_rtmp::messages::{RtmpMessage, MessagePayload, PeerBandwidthLimitType};
use rml_rtmp::time::RtmpTimestamp;
use std::iter::Iterator;

struct Decoder {
    decoder: ChunkDeserializer,
    buffer: Bytes
}

impl Decoder {
    pub fn new () -> Self {
        Self {
            decoder: ChunkDeserializer::new(),
            buffer: Bytes::new(),
        }
    }

    pub fn process(&mut self, buffer: &Bytes) {
        self.buffer = buffer.clone();
    }

    fn size (&mut self, size: u32) {
        self.decoder.set_max_chunk_size(size as usize).unwrap();
    }
}

impl Iterator for Decoder {
    type Item = RtmpMessage;

    #[rustfmt::skip]
    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(Some(payload)) = self.decoder.get_next_message(&self.buffer[..]) {
            if let Ok(message) = payload.to_rtmp_message() {
                if let RtmpMessage::SetChunkSize { size } = message { self.size(size); }
                self.buffer.clear();
                return Some(message);
            }
        }
        
        None
    }
}

/// 处理Rtmp会话信息.
///
/// 解码Rtmp缓冲区并编码Rtmp数据块回复到对端.
pub struct Session {
    decoder: Decoder,
    encoder: ChunkSerializer
}

impl Session {

    /// 创建Rtmp会话信息.
    ///
    /// 创建握手处理类型.
    /// 通过读取 `completed` 字段可获取握手是否完成.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use handshake::Handshake;
    ///
    /// let handshake = Handshake::new();
    /// // handshake.completed
    /// ```
    pub fn new() -> Self {
        Self {
            decoder: Decoder::new(),
            encoder: ChunkSerializer::new(),
        }
    }

    /// 处理Rtmp数据包
    ///
    /// 对缓冲区进行解码，并返回需要回复到对端的数据.
    pub fn process(&mut self, buffer: Bytes) -> Bytes {
        let mut receiver = BytesMut::new();
        
        self.decoder.process(&buffer);
        for message in self.decoder {
            match message {
                RtmpMessage::Amf0Command { command_name, .. } => {
                    if command_name.as_str() == "connect" {
                        if let Some(data) = self.invoke_connect() {
                            receiver.put(data);
                        }
                    }
                },
                _ => (),
            }
        }

        receiver.freeze()
    }

    fn from (&mut self, message: RtmpMessage) -> Option<MessagePayload> {
        let timestamp = RtmpTimestamp { value: 0 };
        match MessagePayload::from_rtmp_message(message, timestamp, 0) {
            Ok(payload) => Some(payload),
            _ => None
        }
    }

    fn invoke_connect (&mut self) -> Option<Bytes> {
        let mut buffer = BytesMut::new();
        let CONNECT_MAGS: Vec<RtmpMessage> = vec![
            RtmpMessage::WindowAcknowledgement { size: 5000000 },
            RtmpMessage::SetPeerBandwidth { size: 5000000, limit_type: PeerBandwidthLimitType::Hard },
            RtmpMessage::SetChunkSize { size: 2048 }
        ];

        for message in CONNECT_MAGS {
            if let Some(payload) = self.from(message) {
                if let Ok(packet) = self.encoder.serialize(&payload, false, false) {
                    buffer.put(packet.bytes.as_slice());
                }
            }
        }

        match &buffer.is_empty() {
            false => Some(buffer.freeze()),
            true => None
        }
    }
}
