// mod.
mod handshake;
mod message;
mod chunk;
mod control;
mod amf;


// use.
use bytes::BytesMut;
use std::sync::mpsc::Sender;
use handshake::Handshake;
use message::Message;


/// # RTMP Control.
pub struct RTMP {
    pub handshake: Handshake,
    pub message: Message,
    pub chunk_size: u64
}


impl RTMP {

    /// # Create RTMP.
    /// 
    pub fn new () -> Self {
        RTMP { 
            handshake: Handshake::new(),
            message: Message::new(),
            chunk_size: 0
        }
    }

    /// # Decoder Bytes.
    /// processing RTMP data.
    /// 
    pub fn decoder(&mut self, bytes: BytesMut, sender: Sender<BytesMut>) {
        let mut bytes_copy = bytes.clone();

        // handshake.
        if self.handshake.completed == false {
            let (back, need) = self.handshake.metch(&bytes_copy);

            // reply or rewrite the cache.
            if need == true {
                sender.send(back).unwrap();
            } else {
                bytes_copy = back;
            }
        }

        // is message
        if self.handshake.completed == true {
            self.message.metch(&bytes_copy);
        }

        println!("bytes_copy len {:?}", &bytes_copy.len());
    }
}