use std::io::Error;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt};
use tokio::net::TcpStream;
use transport::{Transport, Flag};
use balance::Performance;
use bytes::BytesMut;

pub struct State {
    exchange: HashMap<Arc<SocketAddr>, Performance>,
    publish: HashMap<Arc<SocketAddr>, Performance>,
    pull: HashMap<Arc<SocketAddr>, Performance>
}

pub struct Socket {
    stream: TcpStream,
    addr: Arc<SocketAddr>,
    state: Arc<RwLock<State>>,
    transport: Transport,
}

impl Socket {
    pub fn new(stream: TcpStream, addr: SocketAddr, state: Arc<RwLock<State>>) -> Self {
        Self { 
            state,
            stream,
            addr: Arc::new(addr),
            transport: Transport::new(),
        }
    }

    pub async fn process(&mut self) -> Result<(), Error> {
        let mut buffer = [0u8; 2048];
        let size = self.stream.read(&mut buffer).await?;
        self.transport.push(BytesMut::from(&buffer[0..size]));
        while let Some(result) = self.transport.decoder() {
            for (flag, data) in result {
                
            }
        }

        Ok(())
    }

    fn process_avg(&mut self, data: BytesMut) {
        if let Ok(payload) = Transport::parse(data) {
            let performance = Performance::from(payload.data);
            
        }
    }
}
