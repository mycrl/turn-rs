use super::{Tx, Rx};
use std::collections::HashMap;
use bytes::BytesMut;

pub struct Stack {
    video: HashMap<String, BytesMut>,
    audio: HashMap<String, BytesMut>,
    frame: HashMap<String, BytesMut>,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            video: HashMap::new(),
            audio: HashMap::new(),
            frame: HashMap::new(),
        }
    }

    pub fn pull(&mut self, channel: String) {
        self.audio.remove(&channel);
        self.video.remove(&channel);
        self.frame.remove(&channel);
    }
}
