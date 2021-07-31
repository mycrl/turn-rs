use std::sync::mpsc::{
    channel, 
    Receiver, 
    Sender
};

use std::net::{
    UdpSocket,
    SocketAddr
};

use std::{
    io::*,
    sync::Arc,
    ptr
};

pub struct Session<'a> {
    socket: Arc<UdpSocket>,
    reader: Receiver<&'a [u8]>,
    addr: SocketAddr,
}

impl<'a> Session<'a> {
    pub fn new(socket: Arc<UdpSocket>, addr: SocketAddr) -> (Self, Sender<&'a [u8]>) {
        let (sender, reader) = channel();
        (Self { socket, addr, reader }, sender)
    }
}

impl<'a> Read for Session<'a> {
    fn read(&mut self, dst: &mut [u8]) -> Result<usize> {
        let src = self.reader.recv().unwrap();
        if src.len() > dst.len() {
            return Err(Error::new(ErrorKind::WriteZero, "not enough capacity!"));
        }

        unsafe {
            ptr::copy(src.as_ptr(), dst.as_mut_ptr(), src.len());
        }

        Ok(src.len())
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
