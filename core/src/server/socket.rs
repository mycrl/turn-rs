use std::io::Error;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt};
use tokio::net::TcpStream;
use balance::Balance;
use bytes::BytesMut;

pub struct State {
    exchange: HashMap<Arc<SocketAddr>, Balance>,
    publish: HashMap<Arc<SocketAddr>, Balance>,
    pull: HashMap<Arc<SocketAddr>, Balance>
}

pub struct Socket {
    stream: TcpStream,
    state: Arc<RwLock<State>>
}

impl Socket {
    pub fn new(stream: TcpStream, state: Arc<RwLock<State>>) -> Self {
        Self { 
            stream, 
            state
        }
    }

    pub async fn process(&mut self) -> Result<(), Error> {
        let mut buffer = [0u8; 2048];
        let size = self.stream.read(&mut buffer).await?;
        let chunk = BytesMut::from(&buffer[0..size]);

        Ok(())
    }
}
