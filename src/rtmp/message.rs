// use.
use bytes::BytesMut;
use super::chunk::Chunk;
use super::control::Control;
use super::amf::AMF;


/// # RTMP Message.
pub struct Message {
    pub chunk_max_size: u64
}


impl Message {

    /// # Create message.
    /// 
    pub fn new () -> Self {
        Message {
            chunk_max_size: 0
        }
    } 

    /// # Split chunk.
    /// 
    pub fn split_chunk (&mut self, bytes: &BytesMut) -> BytesMut {
        let bytes_copy = bytes.clone();
        let chunk = Chunk::packet(&bytes_copy);
        let mut back_body = BytesMut::new();

        // whether the split is successful.
        if chunk.completed == true {
            let bytes_vec = &bytes_copy.to_vec();
            let offset = 12 + chunk.body_size as usize;
            let (_, body) = bytes_vec.split_at(offset);
            back_body = BytesMut::from(body);
        }

        // check chunk is some.
        if chunk.completed == true {
            match chunk.type_id {
                1 => { Control::create(&chunk.type_id, &chunk.body).match_type(self); },
                20 => { AMF::create(&chunk.type_id, &chunk.body).match_type(); },
                _ => ()
            };
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