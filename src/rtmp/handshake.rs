// use.
use rml_rtmp::handshake::Handshake;
use rml_rtmp::handshake::PeerType; 
use rml_rtmp::handshake::HandshakeProcessResult;


/// # Handshake Type.
#[derive(Debug)]
pub enum HandshakeType {
    Overflow(Vec<u8>),
    Back(Vec<u8>)
}


/// # Handshake Instance.
pub struct Handshakes {
    pub server: Handshake,  // rml_rtmp handshake instance.
    pub completed: bool,  // indicates whether the handshake is complete.
    pub status: u8  // handshake status.
}


impl Handshakes {

    /// # Create Handshake Instance.
    pub fn new () -> Self {
        Handshakes { 
            status: 0, 
            completed: false,
            server: Handshake::new(PeerType::Server)
        }
    }

    /// # Process Handshake Bytes packet.
    ///
    /// TODO:
    /// Since rml_rtmp has an unknown problem, all of it only handles Some.
    pub fn process (&mut self, bytes: Vec<u8>) -> Option<HandshakeType> {
        match self.status {

            // No handshake.
            // handling the client C0+C1 package.
            // this is the default for most client implementations.
            // returns whether you need to reply to the client data.
            0 => match self.server.process_bytes(&bytes) {
                Ok(HandshakeProcessResult::InProgress { response_bytes: bytes }) => {
                    self.status = 1;
                    Some(HandshakeType::Back(bytes))
                }, _ => None 
            },

            // The server has replied to S0+S1+S2.
            // handle the C2 returned by the client.
            // no processing.
            1 => match self.server.process_bytes(&bytes) {
                Ok(HandshakeProcessResult::Completed { response_bytes: _, remaining_bytes: overflow }) => {
                    self.status = 2;
                    self.completed = true;
                    match overflow.len() {
                        0 => None,
                        _ => Some(HandshakeType::Overflow(overflow))
                    }
                }, _ => None
            }, _ => None
        }
    }
}