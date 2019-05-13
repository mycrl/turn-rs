//! 0                   1                   2                   3
//! 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-------+-+-------------+-------------------------------+
//! |F|R|R|R| opcode|M| Payload len |    Extended payload length    |
//! |I|S|S|S|  (4)  |A|     (7)     |            (16/64)            |
//! |N|V|V|V|       |S|             |   (if payload len==126/127)   |
//! | |1|2|3|       |K|             |                               |
//! +-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
//! |    Extended payload length continued, if payload len == 127   |
//! + - - - - - - - - - - - - - - - +-------------------------------+
//! |                               | Masking-key, if MASK set to 1 |
//! +-------------------------------+-------------------------------+
//! |    Masking-key (continued)    |          Payload Data         |
//! +-------------------------------- - - - - - - - - - - - - - - - +
//! :                   Payload Data continued ...                  :
//! + - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
//! |                   Payload Data continued ...                  |
//! +---------------------------------------------------------------+
//! 

// use.
use bytes::BytesMut;


/// # WebSokcket Message Frame.
pub struct Frame {
    pub head: u8, // FIN + RSV + opencode.
    pub length: u8 // Length.
}


impl Frame {

    /// # Create frame.
    pub fn new () -> Self {
        Frame {
            head: 0x82,
            length: 0
        }
    }

    pub fn decode (&mut self, bytes: BytesMut) -> BytesMut {
        self.length = bytes.clone().len() as u8;
        let bytes_vec = bytes.to_vec();
        let body = bytes_vec.as_slice();
        let mut message = vec![];
        message.push(self.head);
        message.push(self.length);
        message.extend_from_slice(body);
        BytesMut::from(message)
    }
}