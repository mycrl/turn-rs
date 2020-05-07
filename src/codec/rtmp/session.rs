use super::State;
use super::State::Callback;
use std::collections::HashMap;
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
    pub fn process(&mut self, buffer: Bytes) -> Option<Bytes> {
        let mut receiver = BytesMut::new();
        let messages = self.parse(&buffer[..]);

        for message in messages {
            match message {
                RtmpMessage::Amf0Command { command_name, .. } => {
                    if command_name.as_str() == "connect" {
                        let results = self.get_connect_messages();
                        if let Some(data) = self.from_message(results) {
                            receiver.put(data);
                        }
                    } else if command_name.as_str() == "createStream" {
                        let results = self.get_create_stream();
                        if let Some(data) = self.from_message(results) {
                            receiver.put(data);
                        }
                    } else if command_name.as_str() == "publish" {
                        let results = self.get_publish();
                        if let Some(data) = self.from_message(results) {
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

    fn get_connect_messages (&mut self) -> Vec<RtmpMessage> {
        let mut fms_version = HashMap::new();
        fms_version.insert("fmsVer".to_string(), Amf0Value::Utf8String("FMS/3,0,1,123".to_string()));
        fms_version.insert("capabilities".to_string(), Amf0Value::Number(31.0));

        let mut connect_info = HashMap::new();
        connect_info.insert("level".to_string(), Amf0Value::Utf8String("status".to_string()));
        connect_info.insert("code".to_string(), Amf0Value::Utf8String("NetConnection.Connect.Success".to_string()));
        connect_info.insert("description".to_string(), Amf0Value::Utf8String("Connection succeeded.".to_string()));
        connect_info.insert("objectEncoding".to_string(), Amf0Value::Number(0.0));

        vec![
            RtmpMessage::WindowAcknowledgement { size: 5000000 },
            RtmpMessage::SetPeerBandwidth {
                size: 5000000,
                limit_type: PeerBandwidthLimitType::Hard,
            },
            RtmpMessage::SetChunkSize { size: 4096 },
            RtmpMessage::Amf0Command { 
                command_name: "_result".to_string(),
                transaction_id: 1.0,
                command_object: Amf0Value::Object(fms_version),
                additional_arguments: vec![ Amf0Value::Object(connect_info) ]
            }
        ]
    }

    fn get_create_stream (&mut self) -> Vec<RtmpMessage> {
        vec![
            RtmpMessage::Amf0Command {
                command_name: "_result".to_string(),
                transaction_id: 4.0,
                command_object: Amf0Value::Null,
                additional_arguments: vec![ Amf0Value::Number(1.0) ]
            }
        ]
    }

    fn get_publish (&mut self) -> Vec<RtmpMessage> {
        let mut publish_info = HashMap::new();
        publish_info.insert("level".to_string(), Amf0Value::Utf8String("status".to_string()));
        publish_info.insert("code".to_string(), Amf0Value::Utf8String("NetStream.Publish.Start".to_string()));
        publish_info.insert("description".to_string(), Amf0Value::Utf8String("Start publishing".to_string()));

        vec![
            RtmpMessage::Amf0Command {
                command_name: "onStatus".to_string(),
                transaction_id: 0.0,
                command_object: Amf0Value::Null,
                additional_arguments: vec![ Amf0Value::Object(publish_info) ]
            }
        ]
    }
}
