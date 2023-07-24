use serde::Serialize;
use parking_lot::Mutex;
use tokio::sync::mpsc::*;
use std::{
    net::SocketAddr,
    collections::*,
    sync::Arc,
};

/// The type of information passed in the monitoring channel
#[derive(Debug, Clone)]
pub enum Stats {
    ReceivedBytes(u16),
    SendBytes(u16),
    ReceivedPkts(u8),
    SendPkts(u8),
}

trait Add {
    fn add(&mut self, num: u64);
}

impl Add for u64 {
    fn add(&mut self, num: u64) {
        *self = self.overflowing_add(num).0;
    }
}

/// Worker independent monitoring statistics
#[derive(PartialEq, Eq, Clone, Copy)]
#[derive(Debug, Default, Serialize)]
pub struct Store {
    pub received_bytes: u64,
    pub send_bytes: u64,
    pub received_pkts: u64,
    pub send_pkts: u64,
}

impl Store {
    /// update status information
    fn change(&mut self, payload: Stats) {
        match payload {
            Stats::ReceivedBytes(v) => {
                self.received_bytes.add((v / 1024) as u64)
            },
            Stats::SendBytes(v) => self.send_bytes.add((v / 1024) as u64),
            Stats::ReceivedPkts(v) => self.received_pkts.add(v as u64),
            Stats::SendPkts(v) => self.send_pkts.add(v as u64),
        }
    }
}

/// worker cluster monitor
#[derive(Clone)]
pub struct Monitor {
    links: Arc<Mutex<BTreeSet<SocketAddr>>>,
    nodes: Arc<Mutex<HashMap<SocketAddr, Store>>>,
    sender: Sender<(SocketAddr, Stats)>,
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Monitor {
    /// Create a monitoring instance
    pub fn new() -> Self {
        let (sender, mut receiver) = channel(2);
        let nodes: Arc<Mutex<HashMap<SocketAddr, Store>>> = Default::default();

        let nodes_ = nodes.clone();
        tokio::spawn(async move {
            while let Some((addr, payload)) = receiver.recv().await {
                if let Some(store) = nodes_.lock().get_mut(&addr) {
                    store.change(payload);
                }
            }
        });

        Self {
            links: Default::default(),
            sender,
            nodes,
        }
    }

    /// get signal sender
    ///
    /// The signal sender can notify the monitoring instance to update internal
    /// statistics.
    ///
    /// # Example
    ///
    /// ```
    /// use turn_server::server::monitor::*;
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let monitor = Monitor::new();
    ///     let sender = monitor.get_actor();
    ///
    ///     sender.send(addr, Stats::ReceivedBytes(100));
    /// }
    /// ```
    pub fn get_actor(&self) -> MonitorActor {
        MonitorActor {
            sender: self.sender.clone(),
        }
    }

    /// Add an address to the watch list
    ///
    /// # Example
    ///
    /// # Example
    ///
    /// ```
    /// use turn_server::server::monitor::*;
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let monitor = Monitor::new();
    ///
    ///     monitor.set(addr.clone());
    ///     let nodes = monitor.get_nodes(0, 10);
    ///     assert_eq!(nodes, vec![(addr, Store::default())]);
    /// }
    /// ```
    pub fn set(&self, addr: SocketAddr) {
        let mut links = self.links.lock();
        self.nodes.lock().insert(addr, Store::default());
        links.insert(addr);
    }

    /// Remove an address from the watch list
    ///
    /// # Example
    ///
    /// ```
    /// use turn_server::server::monitor::*;
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let monitor = Monitor::new();
    ///
    ///     monitor.set(addr.clone());
    ///     let nodes = monitor.get_nodes(0, 10);
    ///     assert_eq!(nodes, vec![(addr.clone(), Store::default())]);
    ///
    ///     monitor.delete(&addr);
    ///     let nodes = monitor.get_nodes(0, 10);
    ///     assert_eq!(nodes, vec![]);
    /// }
    /// ```
    pub fn delete(&self, addr: &SocketAddr) {
        self.nodes.lock().remove(addr);
        self.links.lock().remove(addr);
    }

    /// Obtain a list of statistics from monitoring
    ///
    /// The obtained list is in the same order as it was added.
    ///
    /// # Example
    ///
    /// ```
    /// use turn_server::server::monitor::*;
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let monitor = Monitor::new();
    ///
    ///     monitor.set(addr.clone());
    ///     let nodes = monitor.get_nodes(0, 10);
    ///     assert_eq!(nodes, vec![(addr, Store::default())]);
    /// }
    /// ```
    pub fn get_nodes(
        &self,
        skip: usize,
        limit: usize,
    ) -> Vec<(SocketAddr, Store)> {
        let links = self.links.lock();
        let nodes = self.nodes.lock();

        let mut ret = Vec::with_capacity(limit);
        for addr in links.iter().skip(skip).take(limit) {
            if let Some(store) = nodes.get(addr) {
                ret.push((*addr, *store));
            }
        }

        ret
    }
}

/// monitor sender
///
/// It is held by each worker, and status information can be sent to the
/// monitoring instance through this instance to update the internal statistical
/// information of the monitor.
#[derive(Clone)]
pub struct MonitorActor {
    sender: Sender<(SocketAddr, Stats)>,
}

impl MonitorActor {
    /// TODO: Delivery of updates is not guaranteed.
    pub fn send(&self, addr: SocketAddr, payload: Stats) {
        let _ = self.sender.try_send((addr, payload));
    }
}
