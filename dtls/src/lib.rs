mod session;

use anyhow::Result;
use session::Session;
use std::{
    collections::HashMap,
    mem::transmute,
    thread
};

use openssl::ssl::{
    SslMethod, 
    SslAcceptor,
    SslFiletype
};

use std::sync::{
    mpsc::Sender,
    Arc
};

use std::net::{
    SocketAddr,
    UdpSocket
};

pub struct Cert<'a> {
    pub private: &'a str,
    pub chain: &'a str
}

pub struct Server {
    buf: Vec<u8>,
    socket: Arc<UdpSocket>,
    acceptor: Arc<SslAcceptor>,
    sessions: HashMap<SocketAddr, Sender<&'static [u8]>>
}

impl Server {
    pub fn new<'a>(addr: SocketAddr, cert: Cert<'a>) -> Result<Self> {
        Ok(Self {
            socket: Arc::new(UdpSocket::bind(addr)?),
            sessions: HashMap::with_capacity(1024),
            acceptor: build_acceptor(cert)?,
            buf: vec![0u8; 1024]
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let (size, addr) = self.socket.recv_from(&mut self.buf)?;
            println!("=====================> {}", size);

            let sender = match self.sessions.get(&addr) {
                Some(s) => s,
                None => {
                    let acceptor = self.acceptor.clone();
                    let (session, sender) = Session::new(self.socket.clone(), addr);
                    thread::spawn(move || {
                        let _ = acceptor.accept(session); 
                    });

                    self.sessions.insert(addr, sender);
                    self.sessions.get(&addr).unwrap()
                }
            };

            println!("start send");
            sender.send(unsafe {
                transmute(&self.buf[0..size])
            }).unwrap()
        }
    }
}

fn build_acceptor<'a>(cert: Cert<'a>) -> Result<Arc<SslAcceptor>> {
    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::dtls())?;
    acceptor.set_private_key_file(cert.private, SslFiletype::PEM)?;
    acceptor.set_certificate_chain_file(cert.chain)?;
    acceptor.check_private_key()?;
    Ok(Arc::new(acceptor.build()))
}
