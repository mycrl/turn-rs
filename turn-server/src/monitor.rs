use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

use ahash::AHashMap;
use serde::Serialize;

#[derive(Serialize)]
pub struct NodeCounts {
    pub received_bytes: usize,
    pub send_bytes: usize,
    pub received_pkts: usize,
    pub send_pkts: usize,
}

/// The type of information passed in the monitoring channel
#[derive(Debug, Clone)]
pub enum Stats {
    ReceivedBytes(usize),
    SendBytes(usize),
    ReceivedPkts(usize),
    SendPkts(usize),
}

#[derive(Default)]
struct Count(AtomicUsize);

impl Count {
    fn add(&self, value: usize) {
        self.0.fetch_add(value, Ordering::Relaxed);
    }

    fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

/// Worker independent monitoring statistics
#[derive(Default)]
struct Counts {
    received_bytes: Count,
    send_bytes: Count,
    received_pkts: Count,
    send_pkts: Count,
}

impl Counts {
    fn add(&self, payload: &Stats) {
        match payload {
            Stats::ReceivedBytes(v) => self.received_bytes.add(*v),
            Stats::ReceivedPkts(v) => self.received_pkts.add(*v),
            Stats::SendBytes(v) => self.send_bytes.add(*v),
            Stats::SendPkts(v) => self.send_pkts.add(*v),
        }
    }
}

/// worker cluster monitor
#[derive(Clone, Default)]
pub struct Monitor(Arc<RwLock<AHashMap<SocketAddr, Counts>>>);

impl Monitor {
    /// get signal sender
    ///
    /// The signal sender can notify the monitoring instance to update internal
    /// statistics.
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_server::monitor::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let monitor = Monitor::default();
    ///     let sender = monitor.get_actor();
    ///
    ///     sender.send(&addr, &[Stats::ReceivedBytes(100)]);
    /// }
    /// ```
    pub fn get_actor(&self) -> MonitorActor {
        MonitorActor(self.0.clone())
    }

    /// Add an address to the watch list
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_server::monitor::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let monitor = Monitor::default();
    ///
    ///     monitor.set(addr.clone());
    ///     assert_eq!(monitor.get(&addr).is_some(), true);
    /// }
    /// ```
    pub fn set(&self, addr: SocketAddr) {
        self.0.write().unwrap().insert(addr, Counts::default());
    }

    /// Remove an address from the watch list
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_server::monitor::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let monitor = Monitor::default();
    ///
    ///     monitor.set(addr.clone());
    ///     assert_eq!(monitor.get(&addr).is_some(), true);
    ///
    ///     monitor.delete(&addr);
    ///     assert_eq!(monitor.get(&addr).is_some(), false);
    /// }
    /// ```
    pub fn delete(&self, addr: &SocketAddr) {
        self.0.write().unwrap().remove(addr);
    }

    /// Obtain a list of statistics from monitoring
    ///
    /// The obtained list is in the same order as it was added.
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_server::monitor::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let monitor = Monitor::default();
    ///
    ///     monitor.set(addr.clone());
    ///     assert_eq!(monitor.get(&addr).is_some(), true);
    /// }
    /// ```
    pub fn get(&self, addr: &SocketAddr) -> Option<NodeCounts> {
        self.0.read().unwrap().get(addr).map(|counts| NodeCounts {
            received_bytes: counts.received_bytes.get(),
            received_pkts: counts.received_pkts.get(),
            send_bytes: counts.send_bytes.get(),
            send_pkts: counts.send_pkts.get(),
        })
    }
}

/// monitor sender
///
/// It is held by each worker, and status information can be sent to the
/// monitoring instance through this instance to update the internal statistical
/// information of the monitor.
#[derive(Clone)]
pub struct MonitorActor(Arc<RwLock<AHashMap<SocketAddr, Counts>>>);

impl MonitorActor {
    pub fn send(&self, addr: &SocketAddr, payload: &[Stats]) {
        if let Some(counts) = self.0.read().unwrap().get(addr) {
            for item in payload {
                counts.add(item);
            }
        }
    }
}
