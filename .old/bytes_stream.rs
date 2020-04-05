use tokio::{ prelude::*, io };
use tokio::net::TcpStream;
use futures::try_ready;
use bytes::{Bytes, BytesMut, BufMut};


pub struct BytesStream {
    socket: TcpStream,
    buf_in: BytesMut,
    buf_out: BytesMut,
}


impl BytesStream {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            buf_in: BytesMut::new(),
            buf_out: BytesMut::new(),
        }
    }

    pub fn fill_write_buffer(&mut self, data: &[u8]) {
        self.buf_out.reserve(data.len());
        self.buf_out.put(data);
    }

    pub fn poll_flush(&mut self) -> Poll<(), io::Error> {
        while !self.buf_out.is_empty() {
            let bytes_written = try_ready!(self.socket.poll_write(&self.buf_out));
            if bytes_written > 0 {
                self.buf_out.split_to(bytes_written);
            }
        }

        Ok(Async::Ready(()))
    }

    fn fill_read_buffer(&mut self) -> Poll<(), io::Error> {
        loop {
            self.buf_in.reserve(4096);
            let bytes_read = try_ready!(self.socket.read_buf(&mut self.buf_in));

            if bytes_read == 0 {
                return Ok(Async::Ready(()));
            }

        }
    }
}


impl Stream for BytesStream {
    type Item = Bytes;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let is_socket_closed = self.fill_read_buffer().unwrap().is_ready();

        let data = self.buf_in.take();
        if !data.is_empty() {
            return Ok(Async::Ready(Some(data.freeze())))
        }

        if is_socket_closed {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}