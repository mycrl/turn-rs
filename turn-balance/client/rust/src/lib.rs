//! This is a simple distributed load balancing suite that contains both clients
//! and servers and supports unlimited cascading.
//!
//!
//! ## Toppology
//!
//! ![topoloy](./topology.webp)
//!
//! > Note that the communication protocol between balances uses udp and does
//! > not retransmit, if a packet is lost the current node will never enter the
//! > candidate again.
//!
//! #### Server
//!
//! You can deploy a Balance server in each region and in the server room where
//! the turn server is located and support unlimited cascading, but require each
//! Balance server to be externally accessible.
//!
//! #### Client
//!
//! The client provides SDKs and libraries that can be embedded into your own
//! applications. You can specify a top-level node on the client and initiate a
//! speed query. The client will first ask the top server, the top server will
//! reply to the client with all the subordinates of the current server, the
//! client will concurrently launch a query after getting the list of
//! subordinates, and the first server that replies will become the top server
//! again, and so on iteratively until the node where the turn server is located
//! is found.
//!
//!
//! #### Usage
//!
//! ```no_run
//! use std::net::SocketAddr;
//! use turn_balance_client::Balance;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = "127.0.0.1:3001".parse::<SocketAddr>().unwrap();
//!     let balance = Balance::new(server).await.unwrap();
//!
//!     if let Ok(node) = balance.probe(10).await {
//!         // node is a socket addr.
//!     }
//! }
//! ```

use std::{
    collections::HashMap,
    io::ErrorKind::ConnectionReset,
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use prost::Message;
use proto::{
    balance_request::Payload, balance_response::Reply, BalanceRequest, BalanceResponse, Host,
};
use tokio::{
    net::UdpSocket,
    sync::{
        oneshot::{self, Sender},
        Mutex,
    },
    time::timeout,
};

mod proto {
    include!(concat!(env!("OUT_DIR"), "/balance.rs"));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceError {
    UdpBindFailed,
    NetError,
    NotRecver,
    Timeout,
}

impl std::error::Error for BalanceError {}

impl std::fmt::Display for BalanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UdpBindFailed => "UdpBindFailed",
                Self::NetError => "NetError",
                Self::NotRecver => "NotRecver",
                Self::Timeout => "Timeout",
            }
        )
    }
}

pub struct Balance {
    id: AtomicU32,
    socket: UdpSocket,
    server: SocketAddr,
    received: Mutex<HashMap<u32, bool>>,
    sender: Mutex<Option<Sender<Result<SocketAddr, BalanceError>>>>,
}

impl Balance {
    pub async fn new(server: SocketAddr) -> Result<Arc<Self>, BalanceError> {
        let this = Arc::new(Self {
            server,
            id: AtomicU32::new(0),
            sender: Default::default(),
            received: Default::default(),
            socket: UdpSocket::bind("0.0.0.0:0")
                .await
                .map_err(|_| BalanceError::UdpBindFailed)?,
        });

        let this_ = Arc::downgrade(&this);
        tokio::spawn(async move {
            let mut buf = [0u8; 40960];
            loop {
                if let Some(this) = this_.upgrade() {
                    if let Ok(size) = this.socket.recv(&mut buf).await {
                        if let Ok(res) = BalanceResponse::decode(&buf[..size]) {
                            if let Some(Reply::Probe(probe)) = res.reply {
                                this.handle_probe(res.id, probe.hosts, probe.turn).await;
                            }
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        Ok(this)
    }

    pub async fn probe(&self, timeout_cut: u8) -> Result<SocketAddr, BalanceError> {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        {
            let _ = self.sender.lock().await.insert(tx);
            let mut received = self.received.lock().await;

            received.clear();
            received.insert(id, false);
        }

        self.send(id, &self.server)
            .await
            .map_err(|_| BalanceError::NetError)?;
        timeout(Duration::from_secs(timeout_cut as u64), rx)
            .await
            .map_err(|_| BalanceError::Timeout)?
            .map_err(|_| BalanceError::NotRecver)?
    }

    async fn send(&self, id: u32, addr: &SocketAddr) -> Result<(), std::io::Error> {
        if let Err(e) = self
            .socket
            .send_to(
                &BalanceRequest {
                    payload: Some(Payload::Probe(())),
                    id,
                }
                .encode_to_vec(),
                addr,
            )
            .await
        {
            if e.kind() != ConnectionReset {
                return Err(e);
            }
        }

        Ok(())
    }

    async fn lookup<T>(&self, id: u32, hosts: T) -> Result<(), std::io::Error>
    where
        T: IntoIterator<Item = SocketAddr>,
    {
        for host in hosts {
            self.send(id, &host).await?;
        }

        Ok(())
    }

    async fn handle_probe(&self, id: u32, hosts: Vec<Host>, turn: Option<Host>) {
        let mut received = self.received.lock().await;
        match received.get_mut(&id) {
            Some(item) if *item == false => {
                *item = true;
            }
            _ => return,
        };

        if hosts.is_empty() {
            if let Some(sender) = self.sender.lock().await.take() {
                if let Some((Ok(ip), port)) =
                    turn.map(|turn| (turn.ip.parse::<IpAddr>(), turn.port as u16))
                {
                    let _ = sender.send(Ok(SocketAddr::new(ip, port)));
                }
            }
        } else {
            let id = self.id.fetch_add(1, Ordering::Relaxed);
            received.insert(id, false);

            if self
                .lookup(
                    id,
                    hosts
                        .into_iter()
                        .map(|host| (host.ip.parse::<IpAddr>(), host.port as u16))
                        .filter(|item| item.0.is_ok())
                        .map(|(ip, port)| SocketAddr::new(ip.unwrap(), port)),
                )
                .await
                .is_err()
            {
                if let Some(sender) = self.sender.lock().await.take() {
                    let _ = sender.send(Err(BalanceError::NotRecver));
                }
            }
        }
    }
}
