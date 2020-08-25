use std::io::Error;
use std::mem::transmute;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt};
use tokio::net::TcpStream;
use transport::{Transport, Event, Payload, Flag};
use balance::Performance;
use bytes::{BytesMut, Buf};

/// 节点类型
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Node {
    Exchange = 0,
    Pubish = 1,
    Pull = 2
}

/// 全局连接状态
/// 
/// * `exchange` 交换中心节点.
/// * `publish` 推流处理节点.
/// * `pull` 拉流处理节点.
pub struct State {
    exchange: HashMap<Arc<SocketAddr>, Option<Performance>>,
    publish: HashMap<Arc<SocketAddr>, Option<Performance>>,
    pull: HashMap<Arc<SocketAddr>, Option<Performance>>
}

/// Tcp Socket
/// 
/// 处理tcp连接数据，并根据注册信息对
/// socket分类，以及处理负载信息.
pub struct Socket {
    node: Option<Node>,
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
            node: None,
            addr: Arc::new(addr),
            transport: Transport::new(),
        }
    }

    pub async fn process(&mut self) -> Result<(), Error> {
        self.stream.read(&mut self.transport.buffer).await?;
        while let Some(result) = self.transport.decoder() {
            for (flag, data) in result {
                if flag == Flag::Control {
                    if let Ok(payload) = Transport::parse(data) {
                        match payload.event {
                            Event::Avg => self.process_avg(payload),
                            Event::Register => self.process_register(payload),
                            _ => Ok(()),
                        }?
                    }
                }
            }
        }

        Ok(())
    }

    fn process_avg(&mut self, payload: Payload) -> Result<(), Error> {
        let performance = Performance::from(payload.data);
        if let Some(node) = self.node {
            if let Ok(mut state) = self.state.write() {
                match node {
                    Node::Exchange => state.exchange.insert(self.addr.clone(), Some(performance)),
                    Node::Pubish => state.publish.insert(self.addr.clone(), Some(performance)),
                    Node::Pull => state.pull.insert(self.addr.clone(), Some(performance))
                }.unwrap();
            }
        }

        Ok(())
    }

    fn process_register(&mut self, mut payload: Payload) -> Result<(), Error> {
        let node = unsafe { transmute::<u8, Node>(payload.data.get_u8()) };
        if let Ok(mut state) = self.state.write() {
            match node {
                Node::Exchange => state.exchange.insert(self.addr.clone(), None),
                Node::Pubish => state.publish.insert(self.addr.clone(), None),
                Node::Pull => state.pull.insert(self.addr.clone(), None)
            }.unwrap();
        }

        self.node = Some(node);
        Ok(())
    }
}
