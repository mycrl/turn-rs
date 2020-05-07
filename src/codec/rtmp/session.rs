use super::State;
use super::State::Callback;
use super::packet::MESSAGES;
use bytes::{BufMut, Bytes, BytesMut};
use rml_rtmp::chunk_io::{ChunkDeserializer, ChunkSerializer};
use rml_rtmp::messages::{MessagePayload, RtmpMessage};
use rml_rtmp::time::RtmpTimestamp;

/// 处理Rtmp会话信息.
///
/// 解码Rtmp缓冲区并编码Rtmp数据块回复到对端.
pub struct Session {
    decoder: ChunkDeserializer,
    encoder: ChunkSerializer,
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
            decoder: ChunkDeserializer::new(),
            encoder: ChunkSerializer::new(),
        }
    }

    /// 处理Rtmp数据包
    ///
    /// 对缓冲区进行解码，并返回需要回复到对端的数据.
    pub fn process(&mut self, buffer: Bytes) -> Option<Bytes> {
        let mut receiver = BytesMut::new();
        let messages = self.parse(&buffer[..]);

        for message in messages {
            match message {
                RtmpMessage::Amf0Command { command_name, .. } => {
                    if command_name.as_str() == "connect" {
                        if let Some(data) = self.invoke_connect() {
                            receiver.put(data);
                        }
                    }
                }
                _ => (),
            }
        }

        match &receiver.is_empty() {
            false => Some(receiver.freeze()),
            true => None,
        }
    }

    fn parse(&mut self, buffer: &[u8]) -> Vec<RtmpMessage> {
        let mut result = Vec::new();
        let mut first = true;

        loop {
            let chunk = if first { buffer } else { &[] };
            if let Ok(Some(payload)) = self.decoder.get_next_message(chunk) {
                if let Ok(message) = payload.to_rtmp_message() {
                    match message {
                        RtmpMessage::SetChunkSize { size } => {
                            self.decoder.set_max_chunk_size(size as usize).unwrap();
                        }
                        _ => {
                            result.push(message);
                        }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }

            first = false;
        }

        result
    }

    fn from(&mut self, message: RtmpMessage) -> Option<MessagePayload> {
        let timestamp = RtmpTimestamp { value: 0 };
        match MessagePayload::from_rtmp_message(message, timestamp, 0) {
            Ok(payload) => Some(payload),
            _ => None,
        }
    }

    fn invoke_connect(&mut self) -> Option<Bytes> {
        let mut buffer = BytesMut::new();

        for message in MESSAGES.iter() {
            if let Some(payload) = self.from(message.clone()) {
                if let Ok(packet) = self.encoder.serialize(&payload, false, false) {
                    buffer.put(packet.bytes.as_slice());
                }
            }
        }

        match &buffer.is_empty() {
            false => Some(buffer.freeze()),
            true => None,
        }
    }
}
