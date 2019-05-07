// use.
use super::message::Message;
use byteorder::ReadBytesExt;
use bytes::BigEndian;
use std::io::Cursor;


/// # Protocol Control Instance.
pub struct Control {
    pub type_id: u8,
    pub body: Vec<u8>
}


impl Control {

    /// # Create control instance.
    /// 
    pub fn create (type_id: &u8, body: &Vec<u8>) -> Self {
        Control {
            body: body.clone(),
            type_id: type_id.clone()
        }
    }

    /// # Match types.
    /// 
    pub fn match_type (&self, message: &mut Message) {
        match self.type_id {
            1 => {
                message.chunk_max_size = Cursor::new(&self.body).read_u32::<BigEndian>().unwrap() as u64;
            },
            _ => ()
        };
    }
}