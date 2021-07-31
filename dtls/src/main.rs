mod server;

use openssl::ssl::{SslMethod, SslConnector};
use std::io::{Read, Write};
use std::net::UdpSocket;
use std::thread;
use server::{
    Server,
    Cert
};

use std::io::*;

#[derive(Debug)]
struct Client {
    inner: UdpSocket
}

impl Client {
    fn new(addr: &str) -> Self {
        let inner = UdpSocket::bind("0.0.0.0:0").unwrap();
        inner.connect(addr).unwrap();
        Self { inner }
    }
}

impl Read for Client {
    fn read(&mut self, dst: &mut [u8]) -> Result<usize> {
        self.inner.recv(dst)
    }
}

impl Write for Client {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.send(buf)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

fn main() {
    thread::spawn(|| {
        Server::new(
            "0.0.0.0:8080".parse().unwrap(), 
            Cert {
                private: "d:/mystical/workflow/mycrl.link.key",
                chain: "d:/mystical/workflow/mycrl.link.pem"
            }
        )
        .unwrap()
        .run()
        .unwrap();
    });
    
    let connector = SslConnector::builder(SslMethod::dtls()).unwrap().build();
    connector.connect("mycrl.link", Client::new("localhost:8080")).unwrap();
}
