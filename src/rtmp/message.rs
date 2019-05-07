// use.
use bytes::BytesMut;
use bytes::BigEndian;
use std::io::Cursor;
use byteorder::ReadBytesExt;
use super::chunk::Chunk;
use super::control::Control;
use super::amf::AMF;


/// # RTMP Message.
pub struct Message {
    // chunk_max_size: u64
}


impl Message {

    /// # Create message.
    /// 
    pub fn new () -> Self {
        Message {
            // chunk_max_size: 0
        }
    } 

    /// # Split chunk.
    /// 
    pub fn split_chunk (&mut self, bytes: &BytesMut) -> BytesMut {
        let bytes_copy = bytes.clone();
        let mut back_body = BytesMut::new();
        let mut chunk = Chunk::new();

        // split packet.
        // whether the split is successful.
        chunk.packet(&bytes_copy);
        if chunk.is_ok == true {
            let bytes_vec = &bytes_copy.to_vec();
            let (_, body) = bytes_vec.split_at(12 + chunk.body_size as usize);
            back_body = BytesMut::from(body);
        }

        // return the rest.
        back_body
    }

    /// # Match bytes.
    /// loop drop bytes.
    pub fn metch (&mut self, bytes: &BytesMut) {
        let mut bytes_copy = bytes.clone();
        while &bytes_copy.len() > &(12 as usize) {
            let body = self.split_chunk(&bytes_copy);
            bytes_copy = body;
        }
    }
}