// mod.
mod handshake;
mod frame;


// use.
use bytes::BytesMut;
use std::sync::mpsc::Sender;
use handshake::Handshake;
use frame::Frame;


/// # WebSocket Server Process.
pub struct WebSocket {
    pub address: String,
    pub sender: Sender<BytesMut>,
    pub handshake: Handshake,
    pub frame: Frame
}


impl WebSocket {

    /// # Create WebSocket Contex.
    pub fn new (address: String, sender: Sender<BytesMut>) -> Self {
        WebSocket { 
            address: address,
            frame: Frame::new(),
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

        // message.
        if self.handshake.completed == true {
            let message = self.frame.decode(BytesMut::from(vec![ 0, 1, 2, 3 ]));
            self.sender.send(message).unwrap();
        }
    }
}