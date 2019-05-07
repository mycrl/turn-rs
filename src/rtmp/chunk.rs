// use.
use byteorder::ReadBytesExt;
use bytes::BytesMut;
use bytes::BigEndian;
use std::io::Cursor;


/// # Chunk Instance.
#[derive(Debug)]
pub struct Chunk {
    pub completed: bool,
    pub chunk_type: u8,
    pub chunk_stream_id: u8,
    pub time_stamp: u64,
    pub body_size: u64,
    pub type_id: u8,
    pub stream_id: u64,
    pub body: Vec<u8>
}


impl Chunk {

    /// # Default chunk instance.
    /// 
    pub fn default () -> Self {
        Chunk {
            completed: false,
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
    pub fn packet (bytes: &BytesMut) -> Self {
        let bytes_vec = bytes.to_vec();
        let mut tail_bytes = vec![];
        let mut chunk = Chunk::default();

        // split header.
        if bytes_vec.len() > 12 {
            let (s_basic_header, s_message) = &bytes_vec.split_at(1);
            let (s_header_timestamp, s_header) = s_message.split_at(3);
            let (s_body_size, s_header_body) = s_header.split_at(3);
            let (s_type_id, s_header_right) = s_header_body.split_at(1);
            let (s_stream_id, s_body_left) = s_header_right.split_at(4);
            
            // rewrite.
            tail_bytes = s_body_left.to_vec();
            chunk.chunk_stream_id = s_basic_header[0];
            chunk.type_id = s_type_id[0];

            // [u8; _] as u64;
            chunk.body_size = Cursor::new(s_body_size).read_u24::<BigEndian>().unwrap() as u64;
            chunk.time_stamp = Cursor::new(s_header_timestamp).read_u24::<BigEndian>().unwrap() as u64;
            chunk.stream_id = Cursor::new(s_stream_id).read_u24::<BigEndian>().unwrap() as u64;
        }

        // check body size.
        if tail_bytes.len() >= chunk.body_size as usize {
            let (body, _) = tail_bytes.split_at(chunk.body_size as usize);
            chunk.body = body.to_vec();
            chunk.completed = true;
        }

        chunk
    }
}