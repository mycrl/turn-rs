// mod.
mod handshake;


// use.
use bytes::BytesMut;
use std::sync::mpsc::Sender;
use handshake::Handshake;


/// # WebSocket Server Process.
pub struct WebSocket {
    pub address: String,
    pub sender: Sender<BytesMut>,
    pub handshake: Handshake
}


impl WebSocket {

    /// # Create WebSocket Contex.
    pub fn new (address: String, sender: Sender<BytesMut>) -> Self {
        WebSocket { 
            address: address,
            sender: Sender::clone(&sender),
            handshake: Handshake::new(Sender::clone(&sender))
        }
    }

    /// # Decoder Bytes.
    /// processing WebSocket data.
    pub fn decoder (&mut self, bytes: BytesMut) {

        // handshake.
        if self.handshake.completed == false {
            self.handshake.process(bytes.to_vec());
        }
    }
}