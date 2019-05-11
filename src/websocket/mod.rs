// use.
use bytes::BytesMut;
use std::sync::mpsc::Sender;


/// # WebSocket Server Process.
pub struct WebSocket {
    pub address: String,
    pub sender: Sender<BytesMut>
}


impl WebSocket {

    /// # Create WebSocket Contex.
    pub fn new (address: String, sender: Sender<BytesMut>) -> Self {
        WebSocket { address, sender }
    }

    pub fn decoder (&mut self, bytes: BytesMut) {
        
    }
}