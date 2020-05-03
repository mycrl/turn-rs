pub mod handshake;
pub mod session;

use handshake::Handshake;
use session::Session;
use bytes::Bytes;

/// 处理结果.
pub enum PorcessResult {
    /// 有未处理完成的数据块.
    Overflow(Bytes),

    /// 有需要回复给对等端的数据块.
    Callback(Bytes),

    /// 清空缓冲区
    /// 用于握手到会话之间的传递
    Empty
}

/// RTMP 协议处理.
///
/// 输入输出TCP数据，整个过程自动完成.
/// 同时返回一些关键性的RTMP消息.
pub struct Rtmp {
    handshake: Handshake,
    session: Session,
}

impl Rtmp {
    pub fn new() -> Self {
        Self {
            handshake: Handshake::new(),
            session: Session::new(),
        }
    }

    pub fn process(&mut self, chunk: Bytes) -> Vec<Bytes> {
        println!("message size {:?}", &chunk.len());
        let mut output = Vec::new();
        let mut message = chunk.clone();

        if !&self.handshake.completed {
            if let Some(results) = self.handshake.process(message.clone()) {
                for value in results {
                    match value {
                        PorcessResult::Callback(data) => {
                            &output.push(data);
                        },
                        PorcessResult::Overflow(data) => {
                            message = data;
                        },
                        PorcessResult::Empty => {
                            message.clear();
                        }
                    }
                }
            }
        }

        if self.handshake.completed && !&message.is_empty() {
            if let Some(results) = self.session.process(message) {
                for value in results {
                    if let PorcessResult::Callback(data) = value {
                        &output.push(data);
                    }
                }
            }
        }

        output
    }
}
