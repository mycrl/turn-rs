use bytes::Bytes;
use rml_rtmp::chunk_io::Packet;
use rml_rtmp::sessions::ServerSession;
use rml_rtmp::sessions::ServerSessionConfig;
use rml_rtmp::sessions::ServerSessionResult;

pub enum SessionResult {
    Callback(Bytes),
}

pub struct Session {
    handle: ServerSession,
    outbounds: Vec<ServerSessionResult>,
}

impl Session {
    pub fn new() -> Self {
        let config = ServerSessionConfig::new();
        let (handle, outbounds) = ServerSession::new(config).unwrap();
        Self { handle, outbounds }
    }

    fn packet_parse(packet: &Packet) -> Bytes {
        Bytes::from(&packet.bytes[..])
    }

    fn process_result(result: &ServerSessionResult) -> Option<SessionResult> {
        match result {
            ServerSessionResult::OutboundResponse(packet) => {
                Some(SessionResult::Callback(Self::packet_parse(packet)))
            }
            ServerSessionResult::RaisedEvent(event) => {
                None
            }
            _ => {
                println!("session result no match");
                None
            }
        }
    }

    pub fn process(&mut self, chunk: &Bytes) -> Vec<SessionResult> {
        let mut session_results = Vec::new();

        if self.outbounds.len() > 0 {
            for result in self.outbounds.iter() {
                if let Some(SessionResult::Callback(back)) = Self::process_result(result) {
                    session_results.push(SessionResult::Callback(back));
                }
            }
        }

        if let Ok(results) = self.handle.handle_input(&chunk[..]) {
            for result in results {
                if let Some(SessionResult::Callback(back)) = Self::process_result(&result) {
                    session_results.push(SessionResult::Callback(back));
                }
            }
        }

        session_results
    }
}
