use bytes::BytesMut;
use bytes::BigEndian;
use std::io::Cursor;
use byteorder::ReadBytesExt;


#[derive(Debug)]
pub struct Chunk {
    pub is_ok: bool,
    pub chunk_type: u8,
    pub chunk_stream_id: u8,
    pub time_stamp: u64,
    pub body_size: u64,
    pub type_id: u8,
    pub stream_id: u64,
    pub body: Vec<u8>
}


impl Chunk {

    /// # Create message.
    /// 
    pub fn new () -> Self {
        Chunk {
            is_ok: false,
            chunk_type: 0,
            chunk_stream_id: 0,
            time_stamp: 0,
            body_size: 0,
            type_id: 0,
            stream_id: 0,
            body: vec![]
        }
    }

    /// # Split message package.
    /// 
    pub fn packet (&mut self, bytes: &BytesMut) {
        let bytes_vec = bytes.to_vec();

        // split header.
        if bytes_vec.len() > 12 {
            let (basic_header, message) = &bytes_vec.split_at(1);
            let (header_timestamp, header) = message.split_at(3);
            let (body_size, header_body) = header.split_at(3);
            let (type_id, header_right) = header_body.split_at(1);
            let (stream_id, body_left) = header_right.split_at(4);
            
            // [u8; _] as u32;
            let u_body_size = Cursor::new(body_size).read_u24::<BigEndian>().unwrap();
            let u_time_stamp = Cursor::new(header_timestamp).read_u24::<BigEndian>().unwrap();
            let u_stream_id = Cursor::new(stream_id).read_u24::<BigEndian>().unwrap();

            // check body size.
            if body_left.len() >= u_body_size as usize {
                self.chunk_stream_id = basic_header[0];
                self.time_stamp = u_time_stamp as u64;
                self.body_size = u_body_size as u64;
                self.stream_id = u_stream_id as u64;
                self.type_id = type_id[0];
                self.is_ok = true;

                let (body, _) = body_left.split_at(self.body_size as usize);
                self.body = body.to_vec();
            }
        }
    }
}