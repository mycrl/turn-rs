use crate::rtmp::Rtmp;
use futures::prelude::*;
use bytes::{Bytes, BytesMut};
use tokio::{net::TcpStream, io::AsyncRead, io::AsyncWrite};

enum State {
    Data(Bytes),
    NotData,
    Close
}

pub struct Socket {
    stream: TcpStream,
    codec: Rtmp
}

impl Socket {
    pub fn new(stream: TcpStream) -> Self {
        Self { 
            stream,
            codec: Rtmp::new()
        }
    }

    #[rustfmt::skip]
    pub fn send(&mut self, data: &[u8]) {
        println!("data send size {:?}", &data.len());
        let mut offset: usize = 0;
        loop {
            match self.stream.poll_write(&data[offset..]) {
                Ok(Async::Ready(size)) => match &data.len() < &offset {
                    true => { offset += size; },
                    false => { break; }
                }, _ => (),
            }
        }
    }

    #[rustfmt::skip]
    fn read(&mut self) -> State {
        println!("read data");
        let mut receiver = [0; 4096];
        match self.stream.poll_read(&mut receiver) {
            Ok(Async::Ready(size)) if size > 0 => State::Data(BytesMut::from(&receiver[0..size]).freeze()),
            Ok(Async::Ready(size)) if size == 0 => State::Close,
            _ => State::NotData
        }
    }

    pub fn flush(&mut self) {
        loop {
            match self.stream.poll_flush() {
                Ok(Async::Ready(_)) => { break; },
                _ => (),
            }
        }
    }
}


impl Future for Socket {
    type Item = ();
    type Error = ();

    #[rustfmt::skip]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let State::Data(buffer) = self.read() {
            let receiver = self.codec.process(buffer);
            self.send(&receiver[..]);
            self.flush();
        }

        Ok(Async::NotReady)
    }
}
