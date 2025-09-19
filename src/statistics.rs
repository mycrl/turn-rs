use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use dashmap::DashMap;
use service::session::Identifier;

/// The type of information passed in the statisticsing channel
#[derive(Debug, Clone, Copy)]
pub enum Stats {
    ReceivedBytes(usize),
    SendBytes(usize),
    ReceivedPkts(usize),
    SendPkts(usize),
    ErrorPkts(usize),
}

pub trait Number {
    fn add(&self, value: usize);
    fn get(&self) -> usize;
}

#[derive(Default)]
pub struct Count(AtomicUsize);

impl Number for Count {
    fn add(&self, value: usize) {
        self.0.fetch_add(value, Ordering::Relaxed);
    }

    fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

/// Worker independent statisticsing statistics
pub struct Counts<T> {
    pub received_bytes: T,
    pub send_bytes: T,
    pub received_pkts: T,
    pub send_pkts: T,
    pub error_pkts: T,
}

impl<T: Number> Counts<T> {
    /// # Example
    ///
    /// ```
    /// use turn_server::statistics::*;
    ///
    /// let counts = Counts {
    ///     received_bytes: Count::default(),
    ///     send_bytes: Count::default(),
    ///     received_pkts: Count::default(),
    ///     send_pkts: Count::default(),
    ///     error_pkts: Count::default(),
    /// };
    ///
    /// counts.add(&Stats::ReceivedBytes(1));
    /// assert_eq!(counts.received_bytes.get(), 1);
    ///
    /// counts.add(&Stats::ReceivedPkts(1));
    /// assert_eq!(counts.received_pkts.get(), 1);
    ///
    /// counts.add(&Stats::SendBytes(1));
    /// assert_eq!(counts.send_bytes.get(), 1);
    ///
    /// counts.add(&Stats::SendPkts(1));
    /// assert_eq!(counts.send_pkts.get(), 1);
    /// ```
    pub fn add(&self, payload: &Stats) {
        match payload {
            Stats::ReceivedBytes(v) => self.received_bytes.add(*v),
            Stats::ReceivedPkts(v) => self.received_pkts.add(*v),
            Stats::SendBytes(v) => self.send_bytes.add(*v),
            Stats::SendPkts(v) => self.send_pkts.add(*v),
            Stats::ErrorPkts(v) => self.error_pkts.add(*v),
        }
    }
}

/// worker cluster statistics
#[derive(Clone)]
pub struct Statistics(Arc<DashMap<Identifier, Counts<Count>>>);

impl Default for Statistics {
    #[cfg(feature = "rpc")]
    fn default() -> Self {
        Self(Arc::new(DashMap::with_capacity(1024)))
    }

    // There's no need to take up so much memory when you don't have stats enabled.
    #[cfg(not(feature = "rpc"))]
    fn default() -> Self {
        Self(Default::default())
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
    /// use codec::message::attributes::Transport;
    /// use service::session::Identifier;
    ///
    /// let statistics = Statistics::default();
    /// let sender = statistics.get_reporter(Transport::UDP);
    ///
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// sender.send(&addr, &[Stats::ReceivedBytes(100)]);
    /// ```
    pub fn get_reporter(&self) -> StatisticsReporter {
        StatisticsReporter {
            table: self.0.clone(),
        }
    }

    /// Add an address to the watch list
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_server::statistics::*;
    /// use service::session::Identifier;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// statistics.register(addr.clone());
    /// assert_eq!(statistics.get(&addr).is_some(), true);
    /// ```
    pub fn register(&self, addr: Identifier) {
        self.0.insert(
            addr,
            Counts {
                received_bytes: Count::default(),
                send_bytes: Count::default(),
                received_pkts: Count::default(),
                send_pkts: Count::default(),
                error_pkts: Count::default(),
            },
        );
    }

    /// Remove an address from the watch list
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_server::statistics::*;
    /// use service::session::Identifier;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// statistics.register(addr.clone());
    /// assert_eq!(statistics.get(&addr).is_some(), true);
    ///
    /// statistics.unregister(&addr);
    /// assert_eq!(statistics.get(&addr).is_some(), false);
    /// ```
    pub fn unregister(&self, addr: &Identifier) {
        self.0.remove(addr);
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
    /// use service::session::Identifier;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// statistics.register(addr.clone());
    /// assert_eq!(statistics.get(&addr).is_some(), true);
    /// ```
    pub fn get(&self, addr: &Identifier) -> Option<Counts<usize>> {
        self.0.get(addr).map(|counts| Counts {
            received_bytes: counts.received_bytes.get(),
            received_pkts: counts.received_pkts.get(),
            send_bytes: counts.send_bytes.get(),
            send_pkts: counts.send_pkts.get(),
            error_pkts: counts.error_pkts.get(),
        })
    }
}

/// statistics reporter
///
/// It is held by each worker, and status information can be sent to the
/// statisticsing instance through this instance to update the internal
/// statistical information of the statistics.
#[derive(Clone)]
#[allow(unused)]
pub struct StatisticsReporter {
    table: Arc<DashMap<Identifier, Counts<Count>>>,
}

impl StatisticsReporter {
    #[allow(unused_variables)]
    pub fn send(&self, addr: &Identifier, reports: &[Stats]) {
        #[cfg(feature = "rpc")]
        {
            if let Some(counts) = self.table.get(addr) {
                for item in reports {
                    counts.add(item);
                }
            }
        }
    }
}
