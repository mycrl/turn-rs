use bytes::Bytes;
use rml_rtmp::chunk_io::Packet;
use rml_rtmp::sessions::ServerSession;
use rml_rtmp::sessions::ServerSessionConfig;
use rml_rtmp::sessions::ServerSessionError;
use rml_rtmp::sessions::ServerSessionEvent;
use rml_rtmp::sessions::ServerSessionResult;

pub enum SessionResult {
    Callback(Bytes),
}

pub struct Session {
    handle: ServerSession,
}

impl Session {
    pub fn new() -> Self {
        let config = ServerSessionConfig::new();
        let (handle, _) = ServerSession::new(config).unwrap();
        Self { handle }
    }

    fn packet_parse(packet: Packet) -> Bytes {
        Bytes::from(&packet.bytes[..])
    }

    fn accept_request(&mut self, request_id: u32) -> Result<Vec<Bytes>, ServerSessionError> {
        let mut results = Vec::new();
        for result in self.handle.accept_request(request_id)? {
            if let ServerSessionResult::OutboundResponse(packet) = result {
                results.push(Self::packet_parse(packet));
            }
        }

        Ok(results)
    }

    fn events_match(
        &mut self,
        event: ServerSessionEvent,
    ) -> Result<Vec<Bytes>, ServerSessionError> {
        Ok(match event {
            ServerSessionEvent::ConnectionRequested { request_id, .. } => {
                self.accept_request(request_id)?
            }
            ServerSessionEvent::PublishStreamRequested { request_id, .. } => {
                self.accept_request(request_id)?
            }
            ServerSessionEvent::PlayStreamRequested { request_id, .. } => {
                self.accept_request(request_id)?
            }
            _ => vec![],
        })
    }

    fn process_result(
        &mut self,
        result: ServerSessionResult,
    ) -> Result<Option<SessionResult>, ServerSessionError> {
        Ok(match result {
            ServerSessionResult::OutboundResponse(packet) => {
                Some(SessionResult::Callback(Self::packet_parse(packet)))
            }
            ServerSessionResult::RaisedEvent(_event) => {
                // for value in self.events_match(event)? {

                // }

                None
            }
            _ => {
                println!("session result no match");
                None
            }
        })
    }

    pub fn process(&mut self, chunk: Bytes) -> Result<Vec<SessionResult>, ServerSessionError> {
        let mut session_results = Vec::new();
        for v in self.handle.handle_input(&chunk[..])? {
            if let Some(SessionResult::Callback(back)) = self.process_result(v)? {
                session_results.push(SessionResult::Callback(back));
            }
        }

        Ok(session_results)
    }
}
