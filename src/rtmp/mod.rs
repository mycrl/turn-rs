// mod.
mod handshake;


// use.
use bytes::BytesMut;
use std::sync::mpsc::Sender;
use handshake::Handshake;


/// # RTMP Control.
pub struct RTMP {
    pub handshake: Handshake
}


impl RTMP {

    /// # Create RTMP.
    /// 
    pub fn new () -> Self {
        RTMP { handshake: Handshake::new() }
    }

    /// # Decoder Bytes.
    /// processing RTMP data.
    /// 
    pub fn decoder(&mut self, bytes: BytesMut, sender: Sender<BytesMut>) {

        // check if you need to handle the handshake.
        if self.handshake.types == false {
            let (is_handshake_type, is_handshake_back) = self.handshake.then(&bytes);
            if is_handshake_type {
                if is_handshake_back {
                    let package = self.handshake.created();
                    let body = BytesMut::from(package);
                    sender.send(body).unwrap();
                }
            }
        }
    }
}