use bytes::Bytes;
use rml_rtmp::handshake::Handshake as Handshakes;
use rml_rtmp::handshake::HandshakeProcessResult;
use rml_rtmp::handshake::PeerType;

pub enum HandshakeResult {
    Overflow(Bytes),
    Callback(Bytes),
}

pub struct Handshake {
    handle: Handshakes,
    pub completed: bool
}

impl Handshake {
    pub fn new() -> Self {
        Self {
            handle: Handshakes::new(PeerType::Server),
            completed: false
        }
    }

    pub fn process(&mut self, chunk: &Bytes) -> Vec<HandshakeResult> {
        let mut results = Vec::new();

        if let Ok(result) = self.handle.process_bytes(&chunk[..]) {
            match result {
                HandshakeProcessResult::InProgress { response_bytes } => {
                    if response_bytes.len() > 0 {
                        let buf = Bytes::from(&response_bytes[..]);
                        results.push(HandshakeResult::Callback(buf));
                    }
                }
                HandshakeProcessResult::Completed {
                    response_bytes,
                    remaining_bytes,
                } => {
                    self.completed = true;

                    if response_bytes.len() > 0 {
                        let buf = Bytes::from(&response_bytes[..]);
                        results.push(HandshakeResult::Callback(buf));
                    }

                    if remaining_bytes.len() > 0 {
                        let buf = Bytes::from(&remaining_bytes[..]);
                        results.push(HandshakeResult::Overflow(buf));
                    }
                }
            };
        }

        results
    }
}
