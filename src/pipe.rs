use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::sync::mpsc;
use crate::distributor::Control;


pub struct Channel {
    pub tx: Sender<Control>,
    pub rx: Receiver<Control>
}


pub struct Pipe {
    pub producer: Channel,
    pub consumer: Channel
}


impl Pipe {
    pub fn new () -> Self {
        let (producer_tx, producer_rx) = mpsc::channel();
        let (consumer_tx, consumer_rx) = mpsc::channel();
        let producer = Channel { tx: producer_tx, rx: producer_rx };
        let consumer = Channel { tx: consumer_tx, rx: consumer_rx };
        Pipe { producer, consumer }
    }
}