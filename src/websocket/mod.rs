// use.
use bytes::BytesMut;
use std::sync::mpsc::Sender;
use crate::distributor::Codec;


/// # WebSocket Server Process.
pub struct WebSocket {
    pub address: String,
    pub sender: Sender<BytesMut>
}


impl Codec for WebSocket {

    /// # Create WebSocket Contex.
   fn new (address: String, sender: Sender<BytesMut>) -> Self {
        WebSocket { address, sender }
    }

   fn decoder (&mut self, bytes: BytesMut) {
        
    }
}