use std::collections::HashMap;
use tokio::sync::mpsc;
use bytes::BytesMut;
use std::sync::Arc;

/// 事件
pub enum Event {
    Subscribe(String, Tx),
    Publish(String, Rx),
    Bytes(Arc<BytesMut>),
}

/// 事件传递通道
pub type Rx = mpsc::UnboundedReceiver<Event>;
pub type Tx = mpsc::UnboundedSender<Event>;


/// 核心
pub struct Core {
    publish: HashMap<String, Rx>,
    pull: HashMap<String, Vec<Tx>>,
    frame: HashMap<String, BytesMut>
}
