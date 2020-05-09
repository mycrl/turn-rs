pub mod handshake;
pub mod session;

use super::{Codec, Packet, Transport};
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
    session: Session,
    transport: Transport
}

impl Default for Rtmp {
    /// 创建Rtmp处理程序.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rtmp::Rtmp;
    ///
    /// Server::default();
    /// ```
    fn default() -> Self {
        Self {
            handshake: Handshake::new(),
            session: Session::new(),
            transport: Transport::new(),
        }
    }
}

impl Codec for Rtmp {
    fn parse (&mut self, chunk: Bytes) -> Vec<Packet> {
        let mut buffer = BytesMut::from(&chunk[..]);
        let mut receiver = Vec::new();

        if self.handshake.completed == false {
            if let Some(states) = self.handshake.process(chunk) {
                for state in states {
                    match state {
                        State::Overflow(overflow) => {
                            buffer = BytesMut::from(&overflow[..]);
                        },
                        State::Callback(callback) => {
                            receiver.push(Packet::Tcp(callback));
                        }
                        State::Empty => {
                            buffer.clear();
                        },
                        _ => ()
                    }
                }
            }
        }

        if self.handshake.completed == true && buffer.is_empty() == false {
            if let Some(states) = self.session.process(buffer.freeze()) {
                for state in states {
                    match state {
                        State::Callback(data) => {
                            receiver.push(Packet::Tcp(data));
                        },
                        State::Media(media) => {
                            receiver.push(Packet::Udp(match media {
                                Media::Video(data) => {
                                    self.transport.packet(data, 0u8).unwrap()
                                },
                                Media::Audio(data) => {
                                    self.transport.packet(data, 1u8).unwrap()
                                }
                            }));
                        },
                        _ => ()
                    }
                } 
            }
        }

        receiver
    }
}
