mod session;

use anyhow::Result;
use bytes::BytesMut;
use session::Session;
use std::{
    collections::HashMap,
    mem::transmute
};

use openssl::ssl::{
    SslMethod, 
    SslAcceptor, 
    SslStream, 
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
    buffer: BytesMut,
    socket: Arc<UdpSocket>,
    acceptor: Arc<SslAcceptor>,
    sessions: HashMap<SocketAddr, (
        Sender<&'static [u8]>,
        SslStream<Session<'static>>
    )>
}

impl Server {
    pub fn new<'a>(addr: SocketAddr, cert: Cert<'a>) -> Result<Self> {
        Ok(Self {
            socket: Arc::new(UdpSocket::bind(addr)?),
            sessions: HashMap::with_capacity(1024),
            buffer: BytesMut::with_capacity(2048),
            acceptor: build_acceptor(cert)?
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            println!("start loop");
            unsafe { self.buffer.set_len(0) }
            let (size, addr) = self.socket.recv_from(&mut self.buffer[..])?;
            println!("=====================> {}", size);

            let (sender, _) = match self.sessions.get(&addr) {
                Some(s) => s,
                None => {
                    let (session, sender) = Session::new(self.socket.clone(), addr);
                    if let Ok(stream) = self.acceptor.accept(session) {
                        self.sessions.insert(addr, (sender, stream)).unwrap();
                        self.sessions.get(&addr).unwrap()
                    } else {
                        continue;
                    }
                }
            };

            sender.send(unsafe {
                transmute(&self.buffer[0..size])
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
