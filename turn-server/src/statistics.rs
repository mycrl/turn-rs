use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread::{self, sleep},
    time::Duration,
};

use ahash::AHashMap;
use parking_lot::RwLock;

use crate::config::Transport;

/// [issue](https://github.com/mycrl/turn-rs/issues/101)
///
/// Integrated Prometheus Metrics Exporter
#[cfg(feature = "prometheus")]
pub mod prometheus {
    use anyhow::Result;
    use lazy_static::lazy_static;
    use prometheus::{register_int_counter, Encoder, IntCounter, TextEncoder};

    use super::Stats;
    use crate::config::Transport;

    // The `register_int_counter` macro would be too long if written out in full,
    // with too many line breaks after formatting, and this is wrapped directly into
    // a macro again.
    macro_rules! counter {
        ($prefix:expr, $operation:expr, $dst:expr) => {
            register_int_counter!(
                format!("{}_{}_{}", $prefix, $operation, $dst),
                format!("The {} amount of {} {}", $prefix, $dst, $operation)
            )
        };
    }

    struct Statistics {
        received_bytes: IntCounter,
        send_bytes: IntCounter,
        received_pkts: IntCounter,
        send_pkts: IntCounter,
        error_pkts: IntCounter,
    }

    impl Statistics {
        fn new(prefix: &str) -> Result<Self> {
            Ok(Statistics {
                received_bytes: counter!(prefix, "received", "bytes")?,
                send_bytes: counter!(prefix, "sent", "bytes")?,
                received_pkts: counter!(prefix, "received", "packets")?,
                send_pkts: counter!(prefix, "sent", "packets")?,
                error_pkts: counter!(prefix, "error", "packets")?,
            })
        }

        fn inc(&self, payload: &Stats) {
            match payload {
                Stats::ReceivedBytes(v) => self.received_bytes.inc_by(*v),
                Stats::ReceivedPkts(v) => self.received_pkts.inc_by(*v),
                Stats::SendBytes(v) => self.send_bytes.inc_by(*v),
                Stats::SendPkts(v) => self.send_pkts.inc_by(*v),
                Stats::ErrorPkts(v) => self.error_pkts.inc_by(*v),
            }
        }
    }

    /// Summarized metrics data for Global/TCP/UDP.
    pub struct Metrics {
        total: Statistics,
        tcp: Statistics,
        udp: Statistics,
    }

    impl Default for Metrics {
        fn default() -> Self {
            Self::new().expect("Unable to initialize Prometheus metrics data!")
        }
    }

    impl Metrics {
        fn new() -> Result<Self> {
            Ok(Self {
                total: Statistics::new("total")?,
                tcp: Statistics::new("tcp")?,
                udp: Statistics::new("udp")?,
            })
        }

        pub fn inc(&self, transport: Transport, payload: &Stats) {
            self.total.inc(payload);

            if transport == Transport::TCP {
                self.tcp.inc(payload);
            } else {
                self.udp.inc(payload);
            }
        }
    }

    lazy_static! {
        pub static ref METRICS: Metrics = Metrics::default();
    }

    /// Generate prometheus metrics data that externally needs to be exposed to
    /// the `/metrics` route.
    pub fn generate_metrics(buf: &mut Vec<u8>) -> Result<()> {
        TextEncoder::new().encode(&prometheus::gather(), buf)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SessionCounts {
    pub received_bytes: u64,
    pub send_bytes: u64,
    pub received_pkts: u64,
    pub send_pkts: u64,
    pub error_pkts: u64,
}

/// The type of information passed in the statisticsing channel
#[derive(Debug, Clone, Copy)]
pub enum Stats {
    ReceivedBytes(u64),
    SendBytes(u64),
    ReceivedPkts(u64),
    SendPkts(u64),
    ErrorPkts(u64),
}

#[derive(Default)]
struct Count(AtomicU64);

impl Count {
    fn add(&self, value: u64) {
        self.0.fetch_add(value, Ordering::Relaxed);
    }

    fn get(&self) -> u64 {
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
    error_pkts: Count,
}

impl Counts {
    fn add(&self, payload: &Stats) {
        match payload {
            Stats::ReceivedBytes(v) => self.received_bytes.add(*v),
            Stats::ReceivedPkts(v) => self.received_pkts.add(*v),
            Stats::SendBytes(v) => self.send_bytes.add(*v),
            Stats::SendPkts(v) => self.send_pkts.add(*v),
            Stats::ErrorPkts(v) => self.error_pkts.add(*v),
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
        thread::spawn(move || {
            while let Some(map) = map_.upgrade() {
                let _ = map.read().iter().for_each(|(_, it)| it.clear());
                sleep(Duration::from_secs(1));
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
    pub fn get_reporter(&self) -> StatisticsReporter {
        StatisticsReporter(self.0.clone())
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
        self.0.write().insert(addr, Counts::default());
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
        self.0.write().remove(addr);
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
    pub fn get(&self, addr: &SocketAddr) -> Option<SessionCounts> {
        self.0.read().get(addr).map(|counts| SessionCounts {
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
pub struct StatisticsReporter(Arc<RwLock<AHashMap<SocketAddr, Counts>>>);

impl StatisticsReporter {
    #[allow(unused_variables)]
    pub fn send(&self, transport: Transport, addr: &SocketAddr, reports: &[Stats]) {
        #[cfg(feature = "prometheus")]
        {
            use self::prometheus::METRICS;

            for report in reports {
                METRICS.inc(transport, report);
            }
        }

        if let Some(counts) = self.0.read().get(addr) {
            for item in reports {
                counts.add(item);
            }
        }
    }
}
