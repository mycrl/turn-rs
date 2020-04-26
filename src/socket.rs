use bytes::BufMut;
use bytes::BytesMut;
use futures::try_ready;
use tokio::io::Error;
use tokio::net::TcpStream;
use tokio::prelude::*;
use crate::rtmp::Rtmp;

pub struct Socket {
    socket: TcpStream,
    input: BytesMut,
    output: BytesMut,
    rtmp: Rtmp
}

impl Socket {
    
    pub fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            input: BytesMut::new(),
            output: BytesMut::new(),
            rtmp: Rtmp::new(),
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        self.output.reserve(data.len());
        self.output.put(data);
    }

    pub fn read(&mut self, size: usize) -> Poll<(), Error> {
        loop {
            self.input.reserve(size);
            let result = self.socket.read_buf(&mut self.input);
            let bytes_read = try_ready!(result);
            if bytes_read == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }

    pub fn flush(&mut self) -> Poll<(), Error> {
        while !self.output.is_empty() {
            let result = self.socket.poll_write(&self.output);
            let bytes_written = try_ready!(result);
            if bytes_written > 0 {
                self.output.split_to(bytes_written);
            }
        }

        Ok(Async::Ready(()))
    }
}

impl Future for Socket {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let closed = self.read(4096).unwrap().is_ready();
        let result = self.input.take();

        if !result.is_empty() {
            let data = result.freeze();
            for back in self.rtmp.process(data) {
                self.write(&back[..]);
                self.flush().unwrap();
            }
        }

        if closed {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}
