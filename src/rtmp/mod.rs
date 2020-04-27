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
        let mut output = Vec::new();

        if !&self.handshake.completed {
            if let Some(results) = self.handshake.process(chunk.clone()) {
                for value in results {
                    if let PorcessResult::Callback(data) = value {
                        &output.push(data);
                    }
                }
            }
        }

        if self.handshake.completed {
            if let Some(results) = self.session.process(chunk) {
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
