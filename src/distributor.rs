// use.
use bytes::BytesMut;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use futures::future::lazy;
use crate::pool::Pool;
use crate::pool::CacheBytes;


/// # Media Data Transmission Interface.
#[derive(Clone)]
pub struct Matedata {
    pub name: String,
    pub key: String,
    pub value: CacheBytes
}


#[derive(Clone)]
pub struct Crated {
    pub name: String,
    pub key: String 
}


#[derive(Clone)]
pub enum DataType {
    Matedata(Matedata),
    BytesMut(BytesMut),
    Crated(Crated)
}


pub struct Channel {
    pub tx: Sender<DataType>,
    pub rx: Receiver<DataType>
}


/// # Flow Distributor.
pub struct Distributor {
    pub channel: Channel
}


/// # Interface implemented for the encoder.
/// All encoders must implement the same interface, the same behavior.
pub trait Codec {
    fn new (address: String, sender: Sender<BytesMut>) -> Self;
    fn decoder (&mut self, bytes: BytesMut) -> ();
}


impl Distributor {

    /// # Create distributor.
    pub fn new () -> Self {
        let (tx, rx) = channel();
        Distributor { channel: Channel { tx, rx } }
    }

    pub fn work (self) {
        let mut pool = Pool::new();
        tokio::run(lazy(move || {
            for receive in &self.channel.rx {
                match receive {
                    DataType::Matedata(meta) => pool.put(meta.name, meta.key, meta.value),
                    DataType::Crated(crated) => pool.create(crated.name, crated.key),
                    _ => ()
                }
            }

            Ok(())
        }));
    }
}