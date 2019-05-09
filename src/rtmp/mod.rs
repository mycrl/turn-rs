// mod.
mod handshake;
mod session;


// use.
use bytes::BytesMut;
use std::sync::mpsc::Sender;
use handshake::Handshakes;
use handshake::HandshakeType;
use session::Session;


/// # RTMP Control.
pub struct RTMP {
    pub handshake: Handshakes,
    pub session: Session
}


impl RTMP {

    /// # Create RTMP.
    pub fn new (address: String) -> Self {
        let handshake = Handshakes::new();
        let session = Session::new(address);
        RTMP { handshake, session }
    }

    /// # Decoder Bytes.
    /// processing RTMP data.
    pub fn decoder(&mut self, bytes: BytesMut, sender: Sender<BytesMut>) { 
        let mut bytes_copy = bytes.clone().to_vec();

        // handshake.
        if self.handshake.completed == false {
            if let Some(types) = self.handshake.process(bytes_copy.clone()) {
                match types {
                    HandshakeType::Back(x) => { sender.send(BytesMut::from(x)).unwrap(); },
                    HandshakeType::Overflow(x) => { bytes_copy = x; },
                    HandshakeType::Drop => { bytes_copy = vec![]; }
                }
            }
        }

        // message.
        if self.handshake.completed == true && bytes_copy.len() > 0 {
            println!("message {:?}", bytes_copy);
            println!("message len {:?}", bytes_copy.len());
            self.session.process(bytes_copy.clone(), sender);
        }
    }
}