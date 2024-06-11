use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::Duration,
};

use ahash::AHashMap;
use tokio::time::sleep;

#[derive(Debug, Clone, Copy)]
pub struct NodeCounts {
    pub received_bytes: usize,
    pub send_bytes: usize,
    pub received_pkts: usize,
    pub send_pkts: usize,
}

/// The type of information passed in the statisticsing channel
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

    fn set_zero(&self) {
        self.0.fetch_add(0, Ordering::Relaxed);
    }
}

/// Worker independent statisticsing statistics
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

    fn clear(&self) {
        self.received_bytes.set_zero();
        self.received_pkts.set_zero();
        self.send_bytes.set_zero();
        self.send_pkts.set_zero();
    }
}

/// worker cluster statistics
#[derive(Clone)]
pub struct Statistics(Arc<RwLock<AHashMap<SocketAddr, Counts>>>);

impl Default for Statistics {
    fn default() -> Self {
        let map: Arc<RwLock<AHashMap<SocketAddr, Counts>>> = Default::default();
        let map_ = Arc::downgrade(&map);
        tokio::spawn(async move {
            while let Some(map) = map_.upgrade() {
                let _ = map.read().unwrap().iter().for_each(|(_, it)| it.clear());
                sleep(Duration::from_secs(1)).await;
            }
        });

        Self(map)
    }
}

impl Statistics {
    /// get signal sender
    ///
    /// The signal sender can notify the statisticsing instance to update
    /// internal statistics.
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_server::statistics::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let statistics = Statistics::default();
    ///     let sender = statistics.get_actor();
    ///
    ///     sender.send(&addr, &[Stats::ReceivedBytes(100)]);
    /// }
    /// ```
    pub fn get_actor(&self) -> StatisticsActor {
        StatisticsActor(self.0.clone())
    }

    /// Add an address to the watch list
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_server::statistics::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let statistics = Statistics::default();
    ///
    ///     statistics.set(addr.clone());
    ///     assert_eq!(statistics.get(&addr).is_some(), true);
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
    /// use turn_server::statistics::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let statistics = Statistics::default();
    ///
    ///     statistics.set(addr.clone());
    ///     assert_eq!(statistics.get(&addr).is_some(), true);
    ///
    ///     statistics.delete(&addr);
    ///     assert_eq!(statistics.get(&addr).is_some(), false);
    /// }
    /// ```
    pub fn delete(&self, addr: &SocketAddr) {
        self.0.write().unwrap().remove(addr);
    }

    /// Obtain a list of statistics from statisticsing
    ///
    /// The obtained list is in the same order as it was added.
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_server::statistics::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    ///     let statistics = Statistics::default();
    ///
    ///     statistics.set(addr.clone());
    ///     assert_eq!(statistics.get(&addr).is_some(), true);
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

/// statistics sender
///
/// It is held by each worker, and status information can be sent to the
/// statisticsing instance through this instance to update the internal
/// statistical information of the statistics.
#[derive(Clone)]
pub struct StatisticsActor(Arc<RwLock<AHashMap<SocketAddr, Counts>>>);

impl StatisticsActor {
    pub fn send(&self, addr: &SocketAddr, payload: &[Stats]) {
        if let Some(counts) = self.0.read().unwrap().get(addr) {
            for item in payload {
                counts.add(item);
            }
        }
    }
}
