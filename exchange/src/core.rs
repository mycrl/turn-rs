use std::collections::HashMap;
use tokio::sync::watch;
use bytes::BytesMut;

pub type Tx = watch::Sender<BytesMut>;
pub type Rx = watch::Receiver<BytesMut>;

pub struct Line {
    receiver: Rx,
    sender: Tx,
}

pub struct Core {
    lines: HashMap<String, Line>,
}

impl Core {
    pub fn new() -> Self {
        Self {
            lines: HashMap::new(),
        }
    }
}
