use std::collections::HashMap;
use bytes::{BytesMut, BufMut};

pub struct Buffer {
    buffer: HashMap<String, BytesMut>
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            buffer: HashMap::new(),
        }
    }
}
