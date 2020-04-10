use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::io::Error;
use futures::try_ready;
use bytes::BytesMut;
use bytes::BufMut;
use bytes::Bytes;

pub struct Socket {
    socket: TcpStream,
    input: BytesMut,
    output: BytesMut,
}


impl Socket {
    pub fn new (socket: TcpStream) -> Self {
        Self {
            socket,
            input: BytesMut::new(),
            output: BytesMut::new()
        }
    }

    pub fn write (&mut self, data: &[u8]) {
        self.output.reserve(data.len());
        self.output.put(data);
    }

    pub fn read (&mut self, size: usize) -> Poll<(), Error> {
        loop {
            self.input.reserve(size);
            let result = self.socket
                .read_buf(&mut self.input);
            let bytes_read = try_ready!(result);
            if bytes_read == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }

    pub fn flush (&mut self) -> Poll<(), Error> {
        while !self.output.is_empty() {
            let result = self.socket
                .poll_write(&self.output);
            let bytes_written = try_ready!(result);
            if bytes_written > 0 {
                self.output.split_to(bytes_written);
            }
        }

        Ok(Async::Ready(()))
    }
}


impl Stream for Socket {
    type Item = Bytes;
    type Error = ();
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let closed = self.read(4096)
            .unwrap()
            .is_ready();
        let result = self.input.take();

        if !result.is_empty() {
            return Ok(Async::Ready(Some(result.freeze())))
        }

        if closed {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}

impl Future for Socket {
    type Item = ();
    type Error = ();
    fn poll (&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Async::Ready(()))
    } 
}