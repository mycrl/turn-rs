use super::State;
use super::Media;
use super::State::Callback;
use super::message::{CONNECT, CREATE_STREAM, PUBLISH};
use bytes::{BufMut, Bytes, BytesMut};
use rml_rtmp::chunk_io::{ChunkDeserializer, ChunkSerializer};
use rml_rtmp::messages::{MessagePayload, RtmpMessage, PeerBandwidthLimitType};
use rml_rtmp::time::RtmpTimestamp;
use rml_amf0::Amf0Value;

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
    pub fn process(&mut self, buffer: Bytes) -> Option<Vec<State>> {
        let mut receiver = Vec::new();
        for message in self.parse(&buffer[..]) {
            match message {
                RtmpMessage::AudioData { data } => {
                    receiver.push(State::Media(Media::Audio(data)));
                },
                RtmpMessage::VideoData { data } => {
                    receiver.push(State::Media(Media::Video(data)));
                },
                RtmpMessage::Amf0Command { command_name, .. } => {
                    let name = command_name.as_str();
                    if name == "connect" {
                        if let Some(data) = self.from_message(CONNECT.to_vec()) {
                            receiver.push(Callback(data));
                        }
                    } else 
                    if name == "createStream" {
                        if let Some(data) = self.from_message(CREATE_STREAM.to_vec()) {
                            receiver.push(Callback(data));
                        }
                    } else 
                    if name == "publish" {
                        if let Some(data) = self.from_message(PUBLISH.to_vec()) {
                            receiver.push(Callback(data));
                        }
                    }
                },
                _ => (),
            }
        }

        match &receiver.is_empty() {
            false => Some(receiver),
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
                            let timestamp = RtmpTimestamp { value: 0 };
                            self.decoder.set_max_chunk_size(size as usize).unwrap();
                            self.encoder.set_max_chunk_size(size, timestamp).unwrap();
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
        match message.into_message_payload(timestamp, 0) {
            Ok(payload) => Some(payload),
            _ => None,
        }
    }

    fn from_message (&mut self, messages: Vec<RtmpMessage>) -> Option<Bytes> {
        let mut buffer = BytesMut::new();
        for message in messages {
            if let Some(payload) = self.from(message.clone()) {
                if let Ok(packet) = self.encoder.serialize(&payload, false, false) {
                    buffer.put(&packet.bytes[..]);
                }
            }
        }

        match &buffer.is_empty() {
            false => Some(buffer.freeze()),
            true => None,
        }
    }
}
