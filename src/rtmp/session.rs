// use.
use uuid::Uuid;
use rml_rtmp::sessions::ServerSession;
// use.
use rml_rtmp::sessions::ServerSessionConfig;
use rml_rtmp::sessions::ServerSessionResult;
use rml_rtmp::sessions::ServerSessionEvent;
use std::sync::mpsc::Sender;


/// # Client Action Status.
pub enum ClientAction {
    Waiting,
    Publishing(String), // Publishing to a stream key
    Watching { stream_key: String, stream_id: u32 }
}


/// # Server Session Instance.
pub struct Session {
    pub uid: String,
    pub name: String,
    pub address: String,
    pub session: ServerSession,
    pub results: Vec<ServerSessionResult>,
    pub sender: Sender<Vec<u8>>,
    pub current_action: ClientAction
}


impl Session {

    /// # Create a session instance.
    pub fn new (address: String, sender: Sender<Vec<u8>>) -> Self {
        let name = String::new();
        let uid = Uuid::new_v4().to_string();
        let config = ServerSessionConfig::new();
        let current_action = ClientAction::Waiting;
        let (session, results) = ServerSession::new(config).unwrap();
        Session { uid, address, session, results, sender, name, current_action }
    }

    /// # Accept request.
    /// Tells the server session that it should accept an outstanding request.
    pub fn accept_request (&mut self, request_id: u32) {
        match self.session.accept_request(request_id) {
            Ok(results) => self.session_result(results),
            Err(err) => { println!("Accept request err {:?}", err); }
        }
    }
    
    /// Event.
    /// # ConnectionRequested.
    /// The client is requesting a connection on the specified RTMP application name.
    pub fn event_connection_requested (&mut self, request_id: u32, app_name: String) {
        self.name = app_name;
        self.accept_request(request_id);
    }

    /// Event.
    /// # PublishStreamRequested.
    /// The client is requesting a stream key be released for use.
    pub fn event_publish_requested (&mut self, request_id: u32, app_name: String, stream_key: String) {
        // TODO：need append stream in pool.
        self.name = app_name;
        self.current_action = ClientAction::Publishing(stream_key.clone());
        self.accept_request(request_id);
    }

    // TODO：need append stream in pool.
    /// Event.
    /// # PlayStreamRequested.
    /// The client is requesting playback of the specified stream.
    pub fn event_play_requested (&mut self, request_id: u32, app_name: String, stream_key: String, stream_id: u32) {
        self.name = app_name;
        self.current_action = ClientAction::Watching {
            stream_key: stream_key.clone(),
            stream_id
        };

        match self.session.accept_request(request_id) {
            Err(err) => { println!("PlayStreamRequested err {:?}", err); },
            Ok(results) => {

            }
        }
    }

    /// # Process RTMP session event.
    pub fn events_match (&mut self, event: ServerSessionEvent) {
        match event {
            ServerSessionEvent::ConnectionRequested { request_id, app_name } => self.event_connection_requested(request_id, app_name),
            ServerSessionEvent::PublishStreamRequested { request_id, app_name, stream_key, mode: _ } => self.event_publish_requested(request_id, app_name, stream_key),
            ServerSessionEvent::PlayStreamRequested { request_id, app_name, stream_key, start_at: _, duration: _, reset: _, stream_id } => self.event_play_requested(request_id, app_name, stream_key, stream_id),
            // ServerSessionEvent::StreamMetadataChanged {app_name, stream_key, metadata} => {
            //     //self.metadata_received(app_name, stream_key, metadata, server_results);
            // },
            // ServerSessionEvent::VideoDataReceived {app_name: _, stream_key, data, timestamp} => {
            //     //self.audio_video_data_received(stream_key, timestamp, data, ReceivedDataType::Video, server_results);
            // },
            // ServerSessionEvent::AudioDataReceived {app_name: _, stream_key, data, timestamp} => {
            //     //self.audio_video_data_received(stream_key, timestamp, data, ReceivedDataType::Audio, server_results);
            // },
            _ => ()
        }
    }

    /// # handle the response event of the session.
    pub fn session_result (&mut self, results: Vec<ServerSessionResult>) {
        for result in results {
            match result {
                ServerSessionResult::OutboundResponse(packet) => self.sender.send(packet.bytes).unwrap(),
                ServerSessionResult::RaisedEvent(event) => self.events_match(event),
                _ => ()
            }
        };
    }

    /// # processing bytes.
    /// Process the data sent by the client.
    /// trigger the corresponding event.
    pub fn process (&mut self, bytes: Vec<u8>) {
        match self.session.handle_input(bytes.as_slice()) {
            Ok(results) => self.session_result(results), 
            Err(err) => { println!("process err {:?}", err); }
        };
    }
}