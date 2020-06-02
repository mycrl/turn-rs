use super::State;
use super::State::{Callback, Empty, Overflow};
use bytes::BytesMut;
use rml_rtmp::handshake::Handshake as Handshakes;
use rml_rtmp::handshake::HandshakeProcessResult::Completed;
use rml_rtmp::handshake::HandshakeProcessResult::InProgress;
use rml_rtmp::handshake::PeerType;

/// RTMP handshake processing
///
/// Note: Currently, the client handshake is
/// only handled as a server.
pub struct Handshake {
    handshakes: Handshakes,

    /// handshake is completed.
    pub completed: bool,
}

impl Handshake {
    /// Create handshake processing
    ///
    /// Create a handshake processing instance.
    /// You can check whether the handshake is completed by
    /// getting the "completed" field.
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

    /// Handshake processing
    ///
    /// Process TCP data and return data that needs to
    /// be returned or overflow data.
    /// The entire handshake process will be completed automatically.
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
    pub fn process(&mut self, buffer: &[u8]) -> Option<Vec<State>> {
        match self.handshakes.process_bytes(buffer) {
            Ok(InProgress { response_bytes }) => self.inprogress(response_bytes),
            Ok(Completed { response_bytes, remaining_bytes }) => self.completed(response_bytes, remaining_bytes),
            _ => None,
        }
    }

    /// Check the handshake for overflow data
    fn is_overflow(&mut self, overflow: Vec<u8>) -> State {
        match &overflow.is_empty() {
            false => Overflow(BytesMut::from(&overflow[..])),
            true => Empty,
        }
    }

    /// Handling during the handshake
    ///
    /// During the handshake, the handshake return packet will be returned.
    fn inprogress(&mut self, res: Vec<u8>) -> Option<Vec<State>> {
        match &res.is_empty() {
            false => Some(vec![Callback(BytesMut::from(&res[..]))]),
            true => None,
        }
    }

    /// Handling after completion of handshake
    ///
    /// At this point, the handshake is complete.
    /// There may be overflow of unprocessed data, at this time
    /// should continue to be handed over to the next process.
    #[rustfmt::skip]
    fn completed(&mut self, res: Vec<u8>, remain: Vec<u8>) -> Option<Vec<State>> {
        self.completed = true;
        let mut results = Vec::new();
        if !res.is_empty() { results.push(Callback(BytesMut::from(&res[..]))); }
        results.push(self.is_overflow(remain));
        match &results.is_empty() {
            false => Some(results),
            true => None,
        }
    }
}

impl Default for Handshake {
    fn default() -> Self {
        Self::new()
    }
}
