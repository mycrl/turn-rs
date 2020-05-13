pub mod handshake;
pub mod session;
mod message;

use super::{Codec, Packet};
use bytes::{Bytes, BytesMut};
use handshake::Handshake;
use session::Session;

/// 媒体数据.
pub enum Media {
    /// 视频数据.
    Video(Bytes),
    /// 音频数据.
    Audio(Bytes)
}

/// 处理结果.
pub enum State {
    /// 有未处理完成的数据块.
    Overflow(Bytes),
    /// 有需要回复给对等端的数据块.
    Callback(Bytes),
    /// 清空缓冲区
    /// 用于握手到会话之间的传递
    Empty,
    /// 多媒体数据.
    Media(Media)
}

/// Rtmp协议处理.
///
/// 输入输出TCP数据，整个过程自动完成.
/// 同时返回一些关键性的RTMP消息.
pub struct Rtmp {
    handshake: Handshake,
    session: Session
}

impl Rtmp {

    /// 处理Rtmp握手
    /// 
    /// 传入可写的buffer和results，将自动完成.
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use rtmp::Rtmp;
    /// use bytes::BytesMut;
    ///
    /// let mut rtmp = Rtmp::default();
    /// let mut results = Vec::new();
    /// let mut buffer = BytesMut::from(b"");
    /// rtmp.process_handshake(&buffer, &results);
    /// ```
    pub fn process_handshake(&mut self, buffer: &mut BytesMut, receiver: &mut Vec<Packet>) {
        if let Some(states) = self.handshake.process(&buffer) {
            for state in states {
                if let Some(packet) =  self.process_state(state, buffer) {
                    receiver.push(packet);
                }
            }
        }
    }

    /// 处理Rtmp消息
    /// 
    /// 传入可写的buffer和results，将自动完成.
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use rtmp::Rtmp;
    /// use bytes::BytesMut;
    ///
    /// let mut rtmp = Rtmp::default();
    /// let mut results = Vec::new();
    /// let mut buffer = BytesMut::from(b"");
    /// rtmp.process_session(&buffer, &results);
    /// ```
    pub fn process_session(&mut self, buffer: &mut BytesMut, receiver: &mut Vec<Packet>) {
        if let Some(states) = self.session.process(&buffer) {
            for state in states {
                if let Some(packet) =  self.process_state(state, buffer) {
                    receiver.push(packet);
                }
            } 
        }
    }

    /// 处理模块返回的操作结果
    /// 
    /// 结果包含多媒体数据、溢出数据、回调数据、清空控制信息.
    fn process_state(&mut self, state: State, buffer: &mut BytesMut) -> Option<Packet> {
        match state {

            // 音频或者是视频数据
            // 添加flg:
            //   video: 0
            //   audio: 1
            State::Media(media) => match media {
                Media::Video(data) => Some(Packet::Udp(data, 0u8)),
                Media::Audio(data) => Some(Packet::Udp(data, 1u8))
            },

            // 溢出数据
            // 重写缓冲区，将溢出数据传递到下个流程继续处理
            State::Overflow(overflow) => {
                *buffer = BytesMut::from(&overflow[..]);
                None
            },

            // 回调数据
            // 需要发送给对端TcpSocket的数据
            State::Callback(callback) => {
                Some(Packet::Tcp(callback))
            },

            // 特殊需求
            // 清空缓冲区，没有剩下的数据需要处理
            State::Empty => {
                buffer.clear();
                None
            },
        }
    }
}

impl Default for Rtmp {
    fn default() -> Self {
        Self {
            handshake: Handshake::new(),
            session: Session::new(),
        }
    }
}

impl Codec for Rtmp {
    fn parse (&mut self, buffer: &mut BytesMut) -> Vec<Packet> {
        let mut receiver = Vec::new();

        // 握手还未完成
        // 交给握手模块处理Tcp数据        
        if self.handshake.completed == false {
            self.process_handshake(buffer, &mut receiver);
        }

        // 握手已完成
        // 处理Rtmp消息
        if self.handshake.completed == true {
            self.process_session(buffer, &mut receiver);
        }

        receiver
    }
}
