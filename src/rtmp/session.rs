// use.
use uuid::Uuid;
use bytes::Bytes;
use bytes::BytesMut;
use rml_rtmp::sessions::ServerSession;
use rml_rtmp::sessions::ServerSessionConfig;
use rml_rtmp::sessions::ServerSessionResult;
use rml_rtmp::sessions::ServerSessionEvent;
use rml_rtmp::sessions::StreamMetadata;
use rml_rtmp::time::RtmpTimestamp;
use std::sync::mpsc::Sender;


/// # Client Action Status.
pub enum ClientAction {
    Waiting,
    Publishing(String), // Publishing to a stream key
    Watching { stream_key: String, stream_id: u32 }
}


/// # Matedata Type.
pub enum ReceivedDataType {
    Audio, // Audio data.
    Video // Movie data.
}


/// # Server Session Instance.
pub struct Session {
    pub uid: String,
    pub name: String,
    pub address: String,
    pub session: ServerSession,
    pub results: Option<Vec<ServerSessionResult>>,
    pub current_action: ClientAction,
    pub sender: Sender<BytesMut>,
    pub video_sequence_header: Option<Bytes>,
    pub audio_sequence_header: Option<Bytes>
}


impl Session {

    /// # Create a session instance.
    pub fn new (address: String, sender: Sender<BytesMut>) -> Self {
        let uid = Uuid::new_v4().to_string();
        let config = ServerSessionConfig::new();
        let current_action = ClientAction::Waiting;
        let (session, results) = ServerSession::new(config).unwrap();
        let video_sequence_header = None;
        let audio_sequence_header = None;

        Session { 
            uid, address, session,
            current_action, sender, 
            video_sequence_header, 
            audio_sequence_header,
            results: Some(results),
            name: String::new()
        }
    }

    /// # Check if it is video.
    pub fn is_video_sequence_header (data: Bytes) -> bool {
        data.len() >= 2 && data[0] == 0x17 && data[1] == 0x00
    }

    /// # Check if it is audio.
    pub fn is_audio_sequence_header (data: Bytes) -> bool {
        data.len() >= 2 && data[0] == 0xaf && data[1] == 0x00
    }

    /// # Check if it is video frame.
    pub fn is_video_keyframe (data: Bytes) -> bool {
        data.len() >= 2 && data[0] == 0x17 && data[1] != 0x00
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

    /// Event.
    /// # StreamMetadataChanged.
    // The client is changing metadata properties of the stream being published.
    pub fn event_metadata_received (&mut self, app_name: String, stream_key: String, metadata: StreamMetadata) {

    }

    pub fn event_audio_video_data_received (&mut self, app_name: String, stream_key: String, data: Bytes, timestamp: RtmpTimestamp, data_type: ReceivedDataType) {
        match data_type {
            ReceivedDataType::Audio => {
                if Session::is_video_sequence_header(data.clone()) {
                    self.video_sequence_header = Some(data.clone())
                }
            },
            ReceivedDataType::Video => {
                if Session::is_audio_sequence_header(data.clone()) {
                    self.audio_sequence_header = Some(data.clone())
                }
            }
        }
    }

    /// # Process RTMP session event.
    pub fn events_match (&mut self, event: ServerSessionEvent) {
        match event {
            ServerSessionEvent::ConnectionRequested { request_id, app_name } => self.event_connection_requested(request_id, app_name),
            ServerSessionEvent::PublishStreamRequested { request_id, app_name, stream_key, mode: _ } => self.event_publish_requested(request_id, app_name, stream_key),
            ServerSessionEvent::StreamMetadataChanged { app_name, stream_key, metadata } => self.event_metadata_received(app_name, stream_key, metadata),
            ServerSessionEvent::VideoDataReceived { app_name, stream_key, data, timestamp } => self.event_audio_video_data_received(app_name, stream_key, data, timestamp, ReceivedDataType::Video),
            ServerSessionEvent::AudioDataReceived {app_name, stream_key, data, timestamp } => self.event_audio_video_data_received(app_name, stream_key, data, timestamp, ReceivedDataType::Audio),
            _ => ()
        }
    }

    /// # handle the response event of the session.
    pub fn session_result (&mut self, results: Vec<ServerSessionResult>) {
        for result in results {
            match result {
                ServerSessionResult::OutboundResponse(packet) => self.sender_socket(packet.bytes),
                ServerSessionResult::RaisedEvent(event) => self.events_match(event),
                _ => { println!("session result no match"); }
            }
        }
    }

    /// # Write socket.
    /// Send reply data to socket.
    pub fn sender_socket (&mut self, bytes: Vec<u8>) {
        self.sender.send(BytesMut::from(bytes)).unwrap();
    }

    /// # processing bytes.
    /// Process the data sent by the client.
    /// trigger the corresponding event.
    pub fn process (&mut self, bytes: Vec<u8>) {
        if let Some(x) = &self.results {
            for result in x {
                match result {
                    ServerSessionResult::OutboundResponse(packet) => {
                        self.sender.send(BytesMut::from(packet.bytes.clone())).unwrap();
                    },
                    _ => { println!("session result no match"); }
                }
            }
            self.results = None;
        }

        // Takes in bytes that are encoding RTMP chunks and returns 
        // any responses or events that can be reacted to.
        match self.session.handle_input(bytes.as_slice()) {
            Ok(results) => self.session_result(results), 
            Err(err) => { println!("process err {:?}", err); }
        };
    }
}