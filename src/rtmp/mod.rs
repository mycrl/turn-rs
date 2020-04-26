pub mod handshake;
pub mod session;

use bytes::{ Bytes, BytesMut, BufMut };
use handshake::Handshake;
use handshake::HandshakeResult;
use session::SessionResult;
use session::Session;

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
                    HandshakeResult::Overflow(data) => {
                        let mut target = BytesMut::new();
                        target.put(data);
                        target.put(bytes);
                        bytes = target.freeze();
                    }
                    HandshakeResult::Callback(data) => {
                        output.push(data);
                    }
                };
            }
        }

        if self.handshake.completed {
            if let Ok(results) = self.session.process(bytes) {
                for result in  results {
                    match result {
                        SessionResult::Callback(data) => {
                            output.push(data);
                        }
                    };
                }
            }
        }

        output
    }
}
