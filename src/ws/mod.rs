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
            sender: sender,
            handshake: Handshake::new()
        }
    }

    /// # Decoder Bytes.
    /// processing WebSocket data.
    pub fn decoder (&mut self, bytes: BytesMut) {
        self.handshake.process(bytes.to_vec());
    }
}