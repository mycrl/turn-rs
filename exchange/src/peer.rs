use std::collections::HashMap;
use bytes::{BytesMut, BufMut};

pub struct Peer {
    buffer: HashMap<String, BytesMut>
}

impl Peer {
    pub fn new() -> Self {
        Self {
            buffer: HashMap::new(),
        }
    }
}
