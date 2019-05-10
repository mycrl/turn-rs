// mod.
mod handshake;
mod session;


// use.
use bytes::BytesMut;
use std::sync::mpsc::Sender;
use session::Session;
use handshake::Handshakes;
use handshake::HandshakeType;
use crate::distributor::Codec;


/// # RTMP Control.
pub struct Rtmp {
    pub sender: Sender<BytesMut>,
    pub handshake: Handshakes,
    pub session: Session
}


impl Codec for Rtmp {

    /// # Create RTMP.
    fn new (address: String, sender: Sender<BytesMut>) -> Self {
        let session = Session::new(address, Sender::clone(&sender));
        let handshake = Handshakes::new();
        Rtmp { handshake, session, sender }
    }

    /// # Decoder Bytes.
    /// processing RTMP data.
    fn decoder (&mut self, bytes: BytesMut) { 
        let mut bytes_copy = bytes.clone().to_vec();

        // handshake.
        if self.handshake.completed == false {
            if let Some(types) = self.handshake.process(bytes_copy.clone()) {
                match types {
                    HandshakeType::Back(x) => { self.sender.send(BytesMut::from(x)).unwrap(); },
                    HandshakeType::Overflow(x) => { bytes_copy = x; },
                    HandshakeType::Clear => { bytes_copy = vec![]; }
                }
            }
        }

        // message.
        if self.handshake.completed == true && bytes_copy.len() > 0 {
            self.session.process(bytes_copy.clone());
        }
    }
}