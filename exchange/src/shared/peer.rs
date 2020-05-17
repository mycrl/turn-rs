use bytes::BytesMut;
use tokio::sync::watch;

pub type WRx = watch::Receiver<BytesMut>;
pub type WTx = watch::Sender<BytesMut>;

pub struct Peer {
    receiver: WRx,
    sender: WTx,
}

impl Peer {
    pub fn new(data: BytesMut) -> Self {
        let (sender, receiver) = watch::channel(data);
        Self {
            sender,
            receiver,
        }
    }

    pub fn push(&mut self, data: BytesMut) {
        
    }
}
