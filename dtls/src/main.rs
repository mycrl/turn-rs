use openssl::ssl::*;
use anyhow::Result;
use std::net::*;
use std::io::*;

pub struct Socket {
    raw: UdpSocket,
    target: SocketAddr
}

impl Read for Socket {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.raw.recv(buf)
    }
}

impl Write for Socket {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.raw.send_to(buf, self.target)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    let mut socket = UdpSocket::bind("0.0.0.0:0")?;
    let mut ctx = SslContext::builder(SslMethod::dtls())?;
    

    Ok(())
}
