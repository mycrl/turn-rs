use super::PorcessResult;
use super::PorcessResult::Callback;
use rml_rtmp::sessions::{ServerSession, ServerSessionConfig};
use rml_rtmp::sessions::{ServerSessionEvent, ServerSessionResult};
use bytes::{Bytes, BytesMut};

/// 处理 RTMP 会话信息.
pub struct Session {
    session: ServerSession,
}

impl Session {
    pub fn new() -> Self {
        let config = ServerSessionConfig::new();
        let (session, _) = ServerSession::new(config).unwrap();
        Self { session }
    }

    pub fn process(&mut self, chunk: Bytes) -> Option<Vec<PorcessResult>> {
        let mut results = Vec::new();
        match self.session.handle_input(&chunk[..]) {
            Ok(result) => {
                println!("results {:?}", result);
                result
                .iter()
                .enumerate()
                .for_each(|(_, v)| {
                    if let Some(mut values) = self.process_result(v) {
                        results.append(&mut values);
                    }
                })
            },
            Err(e) => {
                println!("{:?}", e);
            }
        };

        match &results.is_empty() {
            false => Some(results),
            true => None
        }
    }

    fn accept (&mut self, request_id: &u32) -> Option<Vec<PorcessResult>> {
        let mut results = Vec::new();
        match self.session.accept_request(*request_id) {
            Ok(result) => result
                .iter()
                .enumerate()
                .for_each(|(_, v)| {
                    if let Some(mut values) = self.process_result(v) {
                        results.append(&mut values);
                    }
                }),
            Err(_) => ()
        };

        match &results.is_empty() {
            false => Some(results),
            true => None
        }
    }

    fn process_event (&mut self, event: &ServerSessionEvent) -> Option<Vec<PorcessResult>> {
        match event {
            ServerSessionEvent::ReleaseStreamRequested { request_id, .. } => self.accept(request_id),
            ServerSessionEvent::PublishStreamRequested { request_id, .. } => self.accept(request_id),
            ServerSessionEvent::PlayStreamRequested { request_id, .. } => self.accept(request_id),
            ServerSessionEvent::ConnectionRequested { request_id, .. } => self.accept(request_id),
            _ => None
        }
    }

    fn process_result (&mut self, result: &ServerSessionResult) -> Option<Vec<PorcessResult>> {
        match result {
            ServerSessionResult::OutboundResponse(packet) => {
                Some(vec![ Callback(BytesMut::from(&packet.bytes[..]).freeze()) ])
            },
            ServerSessionResult::RaisedEvent(event) => {
                self.process_event(event)
            },
            _ => None
        }
    }
}
