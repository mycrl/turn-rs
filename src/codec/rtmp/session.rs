use super::{State, Media};
use super::State::Callback;
use super::message::{CONNECT, CREATE_STREAM, PUBLISH};
use rml_rtmp::chunk_io::{ChunkDeserializer, ChunkSerializer, Packet};
use rml_rtmp::{messages::RtmpMessage, time::RtmpTimestamp};
use bytes::{BufMut, BytesMut};

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
    /// use session::Session;
    ///
    /// Session::new();
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
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use session::Session;
    /// use bytes::Bytes;
    ///
    /// let mut session = Session::new();
    /// session.process(Bytes::from(b""))
    /// ```
    pub fn process(&mut self, buffer: &[u8]) -> Option<Vec<State>> {
        let mut receiver = Vec::new();
        let mut first = true;

        // 循环获取
        // 直到获取失败
        // 拿出所有解码出的RTMP消息
        loop {

            // 库函数要求空调用
            // 所有首次传Tcp数据
            // 后续传空数据
            let chunk = if first { 
                first = false;
                buffer
            } else { 
                &[] 
            };

            // 获取并处理消息
            // 得到调用结果并推入到返回结果列表
            // 只对获取消息做处理
            // 如果获取失败则跳出循环, 表示当前已经消耗完成
            if let Some(message) = self.get_message(chunk) {
                if let Some(state) = self.process_message(message) {
                    receiver.push(state);
                }
            } else {
                break;
            }
        }

        match &receiver.is_empty() {
            false => Some(receiver),
            true => None
        }
    }

    /// 获取消息
    /// 
    /// 消耗Tcp数据并获取Rtmp消息.
    /// 将Result转为Option
    fn get_message(&mut self, chunk: &[u8]) -> Option<RtmpMessage> {
        match self.decoder.get_next_message(chunk) {
            Ok(Some(payload)) => payload.to_rtmp_message().ok(),
            _ => None
        }
    }

    /// 设置最大分片大小
    /// 
    /// 设置解码和编码的最大分片大小.
    /// 分片大小为Rtmp消息的最大长度.
    /// 
    /// TODO: 内部使用了unwrap来解决错误，应该使用其他方法;
    fn set_max_size(&mut self, size: u32) -> Option<State> {
        let timestamp = RtmpTimestamp { value: 0 };
        self.decoder.set_max_chunk_size(size as usize).unwrap();
        self.encoder.set_max_chunk_size(size, timestamp).unwrap();
        None
    }

    /// 处理Rtmp控制消息
    /// 
    /// 目前只处理连接、创建流、推流事件，
    /// 因为目前只用处理基本消息来使推流正常.
    /// 
    /// TODO: 处理不健壮，而且没有将推流的session信息广播出去;
    fn process_command(&mut self, command: &str) -> Option<State> {
        match command {
            "connect" => self.from_message(CONNECT.to_vec()),
            "createStream" => self.from_message(CREATE_STREAM.to_vec()),
            "publish" => self.from_message(PUBLISH.to_vec()),
            _ => None,
        }
    }

    /// 处理Rtmp消息
    /// 
    /// 目前只处理基本消息，
    /// 只为了摘出音视频数据.
    /// 
    /// TODO: 健壮性问题;
    fn process_message(&mut self, message: RtmpMessage) -> Option<State> {
        match message {
            RtmpMessage::SetChunkSize { size } => self.set_max_size(size),
            RtmpMessage::AudioData { data } => Some(State::Media(Media::Audio(data))),
            RtmpMessage::VideoData { data } => Some(State::Media(Media::Video(data))),
            RtmpMessage::Amf0Command { command_name, .. } => self.process_command(command_name.as_str()),
            _ => None,
        }
    }

    /// 创建Rtmp消息
    /// 
    /// 给出Rtmp消息结果，序列化出Rtmp消息数据
    fn from(&mut self, message: RtmpMessage) -> Option<Packet> {
        let timestamp = RtmpTimestamp { value: 0 };
        match message.into_message_payload(timestamp, 0) {
            Ok(payload) => match self.encoder.serialize(&payload, false, false) {
                Ok(packet) => Some(packet),
                _ => None,
            }, _ => None,
        }
    }

    /// 创建Rtmp消息数据
    /// 
    /// 通常一次交互不一定会只有一条Rtmp消息，
    /// 所以此处组装出多条Rtmp消息，并合并返回通过TcpSocket发送到对端.
    fn from_message(&mut self, messages: Vec<RtmpMessage>) -> Option<State> {
        let mut buffer = BytesMut::new();
        for message in messages {
            if let Some(packet) = self.from(message.clone()) {
                buffer.put(&packet.bytes[..]);
            }
        }

        match &buffer.is_empty() {
            false => Some(Callback(buffer.freeze())),
            true => None,
        }
    }
}
