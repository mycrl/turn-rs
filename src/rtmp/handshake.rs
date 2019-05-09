// use.
use rml_rtmp::handshake::Handshake;
use rml_rtmp::handshake::PeerType; 
use rml_rtmp::handshake::HandshakeProcessResult;


/// # Handshake Type.
#[derive(Debug)]
pub enum HandshakeType {
    Overflow(Vec<u8>),
    Back(Vec<u8>),
    Clear
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

    /// Check for overflowed data.
    /// If there is no overflow data, 
    /// there is no need to externally handle this overflow.
    pub fn is_overflow (&mut self, overflow: Vec<u8>) -> Option<HandshakeType> {
        match overflow.len() {
            0 => Some(HandshakeType::Clear),
            _ => Some(HandshakeType::Overflow(overflow))
        }
    }

    /// No handshake.
    /// Handling the client C0+C1 package.
    /// This is the default for most client implementations.
    /// Returns whether you need to reply to the client data.
    pub fn handshake_status_first (&mut self, bytes: &Vec<u8>) -> Option<HandshakeType> {
        match self.server.process_bytes(&bytes) {
            Ok(HandshakeProcessResult::InProgress { response_bytes: bytes }) => {
                self.status = 1;
                Some(HandshakeType::Back(bytes))
            }, _ => None // default.
        }
    }

    /// The server has replied to S0+S1+S2.
    /// Handle the C2 returned by the client.
    /// No processing.
    pub fn handshake_status_two (&mut self, bytes: &Vec<u8>) -> Option<HandshakeType> {
        match self.server.process_bytes(&bytes) {
            Ok(HandshakeProcessResult::Completed { response_bytes: _, remaining_bytes: overflow }) => {
                self.status = 2;
                self.completed = true;
                self.is_overflow(overflow)
            }, _ => None // default.
        }
    }

    /// # Process Handshake Bytes packet.
    /// Assign different processing according to the state of the current handshake.
    pub fn process (&mut self, bytes: Vec<u8>) -> Option<HandshakeType> {
        match self.status {
            0 => self.handshake_status_first(&bytes),
            1 => self.handshake_status_two(&bytes),
            _ => None
        }
    }
}