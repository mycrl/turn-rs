use bytes::Bytes;
use crate::handshake::Handshake;
use crate::handshake::HandshakeResult;
use crate::session::SessionResult;
use crate::session::Session;

pub struct Rtmp {
    handshake: Handshake,
    session: Session
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
        let mut bytes = chunk;

        if self.handshake.completed == false {
            for result in self.handshake.process(&bytes) {
                match result {
                    HandshakeResult::Overflow(mut data) => {
                        data.extend_from_slice(&bytes[..]);
                        bytes = data;
                    }
                    HandshakeResult::Callback(data) => {
                        output.push(data);
                    }
                };
            }
        } else {
            for result in self.session.process(&bytes) {
                match result {
                    SessionResult::Callback(data) => {
                        output.push(data);
                    }
                };
            }
        }

        output
    }
}
