pub mod handshake;
pub mod session;

use bytes::{BufMut, Bytes, BytesMut};
use handshake::Handshake;
use session::Session;

/// 处理结果.
pub enum State {
    /// 有未处理完成的数据块.
    Overflow(Bytes),
    /// 有需要回复给对等端的数据块.
    Callback(Bytes),
    /// 清空缓冲区
    /// 用于握手到会话之间的传递
    Empty,
}

/// Rtmp协议处理.
///
/// 输入输出TCP数据，整个过程自动完成.
/// 同时返回一些关键性的RTMP消息.
pub struct Rtmp {
    handshake: Handshake,
    session: Session,
}

impl Rtmp {
    /// 创建Rtmp处理程序.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rtmp::Rtmp;
    ///
    /// Server::new();
    /// ```
    pub fn new() -> Self {
        Self {
            handshake: Handshake::new(),
            session: Session::new(),
        }
    }

    /// 处理Rtmp数据包
    ///
    /// 对缓冲区进行解码，并返回需要回复到对端的数据.
    pub fn process(&mut self, chunk: Bytes) -> Option<Bytes> {
        println!("on data size {:?}", &chunk.len());
        let mut buffer = BytesMut::from(&chunk[..]);
        let mut receiver = BytesMut::new();

        if self.handshake.completed == false {
            if let Some(states) = self.handshake.process(chunk) {
                for state in states {
                    match state {
                        State::Overflow(overflow) => {
                            buffer = BytesMut::from(&overflow[..]);
                        }
                        State::Callback(callback) => {
                            receiver.put(callback);
                        }
                        State::Empty => {
                            buffer.clear();
                        }
                    }
                }
            }
        }

        if self.handshake.completed == true && buffer.is_empty() == false {
            if let Some(data) = self.session.process(buffer.freeze()) {
                receiver.put(data);
            }
        }

        match &receiver.is_empty() {
            false => Some(receiver.freeze()),
            true => None,
        }
    }
}
