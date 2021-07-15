use std::{io::*, net::SocketAddr};
use tokio::sync::mpsc::Receiver;
use std::net::UdpSocket;
use std::sync::Arc;

pub struct Session<'a> {
    socket: Arc<UdpSocket>,
    reader: Receiver<&'a [u8]>,
    addr: SocketAddr,
}

impl<'a> Read for Session<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let read_buf = self.reader.blocking_recv().unwrap();
        let read_ptr = &mut read_buf as *mut [u8];
        unsafe { std::ptr::write(buf as *mut [u8], read_ptr); }
        Ok(read_buf.len())
    }
}

impl Write for Session<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.socket.send_to(buf, self.addr)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
