// mod.
mod handshake;
mod session;


// use.
use bytes::BytesMut;
use std::sync::mpsc::Sender;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use handshake::Handshakes;
use handshake::HandshakeType;
use session::Session;


/// # RTMP Control.
pub struct RTMP {
    pub handshake: Handshakes,
    pub session: Session,
    pub receiver: Receiver<Vec<u8>>
}


impl RTMP {

    /// # Create RTMP.
    pub fn new (address: String) -> Self {
        let (sender, receiver) = channel();
        let handshake = Handshakes::new();
        let session = Session::new(address, sender);
        RTMP { handshake, session, receiver }
    }

    /// # Decoder Bytes.
    /// processing RTMP data.
    pub fn decoder(&mut self, bytes: BytesMut, sender: Sender<BytesMut>) { 
        let mut bytes_copy = bytes.clone().to_vec();

        // handshake.
        if self.handshake.completed == false {
            if let Some(types) = self.handshake.process(bytes_copy) {
                match types {
                    HandshakeType::Back(bytes) => { sender.send(BytesMut::from(bytes)).unwrap(); },
                    HandshakeType::Overflow(bytes) => { bytes_copy = bytes; }
                }
            }
        } else
        // process message.
        if self.handshake.completed == true {
            self.session.process(bytes_copy);
        }
    }
}