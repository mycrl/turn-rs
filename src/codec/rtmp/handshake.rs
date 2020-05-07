use super::State;
use super::State::{Callback, Empty, Overflow};
use bytes::Bytes;
use rml_rtmp::handshake::Handshake as Handshakes;
use rml_rtmp::handshake::HandshakeProcessResult::Completed;
use rml_rtmp::handshake::HandshakeProcessResult::InProgress;
use rml_rtmp::handshake::PeerType;

/// RTMP 握手处理.
///
/// 注意: 目前只作为服务端处理客户端握手.
pub struct Handshake {
    handshakes: Handshakes,

    /// 握手是否完成.
    pub completed: bool,
}

impl Handshake {
    /// 创建握手处理.
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
            handshakes: Handshakes::new(PeerType::Server),
            completed: false,
        }
    }

    /// 握手处理.
    ///
    /// 处理TCP数据并返回需要返回的数据或者溢出数据.
    /// 整个握手过程将自动完成.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use handshake::Handshake;
    /// use bytes::Bytes;
    ///
    /// let handshake = Handshake::new();
    /// handshake.process(Bytes::from(b"""));
    /// ```
    #[rustfmt::skip]
    pub fn process(&mut self, buffer: Bytes) -> Option<Vec<State>> {
        match self.handshakes.process_bytes(&buffer[..]) {
            Ok(InProgress { response_bytes }) => self.inprogress(response_bytes),
            Ok(Completed { response_bytes, remaining_bytes }) => self.completed(response_bytes, remaining_bytes),
            _ => None,
        }
    }

    /// 检查握手是否有溢出数据.
    fn is_overflow(&mut self, overflow: Vec<u8>) -> State {
        match &overflow.is_empty() {
            false => Overflow(Bytes::from(overflow)),
            true => Empty,
        }
    }

    /// 握手过程中的处理.
    ///
    /// 握手过程中会返回握手回包.
    fn inprogress(&mut self, res: Vec<u8>) -> Option<Vec<State>> {
        match &res.is_empty() {
            false => Some(vec![Callback(Bytes::from(res))]),
            true => None,
        }
    }

    /// 握手完成后的处理.
    ///
    /// 到此为止，握手完成.
    /// 可能还会溢出未处理完成的数据，这时候应该继续交给下个流程进行处理.
    #[rustfmt::skip]
    fn completed(&mut self, res: Vec<u8>, remain: Vec<u8>) -> Option<Vec<State>> {
        self.completed = true;
        let mut results = Vec::new();
        if !res.is_empty() { results.push(Callback(Bytes::from(res))); }
        results.push(self.is_overflow(remain));
        match &results.is_empty() {
            false => Some(results),
            true => None,
        }
    }
}
