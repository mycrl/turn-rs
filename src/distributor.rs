// use.
use bytes::BytesMut;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use crate::pool::Pool;
use crate::pool::CacheBytes;


/// # Media Data Transmission Interface.
#[derive(Clone)]
pub struct Matedata {
    pub name: String,
    pub key: String,
    pub value: CacheBytes
}


pub struct Control {
    pub matedata: Option<Matedata>,
    pub style: String
}


pub struct Channel {
    pub tx: Sender<Matedata>,
    pub rx: Receiver<Matedata>
}


/// # Flow Distributor.
pub struct Distributor {
    pub pool: Pool,
    pub channel: Channel
}


impl Distributor {

    /// # Create distributor.
    pub fn new () -> Self {
        let pool = Pool::new();
        let (tx, rx) = channel();
        Distributor { pool, channel: Channel { tx, rx } }
    }
}