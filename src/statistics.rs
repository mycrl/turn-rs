use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use ahash::AHashMap;
use parking_lot::RwLock;

use crate::{stun::Transport, turn::SessionAddr};

/// [issue](https://github.com/mycrl/turn-rs/issues/101)
///
/// Integrated Prometheus Metrics Exporter
pub mod prometheus {
    use std::sync::LazyLock;

    use anyhow::Result;
    use prometheus::{Encoder, IntCounter, IntGauge, TextEncoder, register_int_counter, register_int_gauge};

    use super::{Counts, Number, Stats};
    use crate::stun::Transport;

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

    pub static METRICS: LazyLock<Metrics> = LazyLock::new(|| Metrics::default());

    /// # Example
    ///
    /// ```
    /// use prometheus::register_int_counter;
    /// use turn_server::statistics::{Number, prometheus::*};
    ///
    /// let count = register_int_counter!("test", "test").unwrap();
    ///
    /// count.add(1);
    /// assert_eq!(count.get(), 1);
    ///
    /// count.add(1);
    /// assert_eq!(count.get(), 2);
    /// ```
    impl Number for IntCounter {
        fn add(&self, value: usize) {
            self.inc_by(value as u64);
        }

        fn get(&self) -> usize {
            self.get() as usize
        }
    }

    impl Counts<IntCounter> {
        fn new(prefix: &str) -> Result<Self> {
            Ok(Self {
                received_bytes: counter!(prefix, "received", "bytes")?,
                send_bytes: counter!(prefix, "sent", "bytes")?,
                received_pkts: counter!(prefix, "received", "packets")?,
                send_pkts: counter!(prefix, "sent", "packets")?,
                error_pkts: counter!(prefix, "error", "packets")?,
            })
        }
    }

    /// Summarized metrics data for Global/TCP/UDP.
    pub struct Metrics {
        pub allocated: IntGauge,
        pub total: Counts<IntCounter>,
        pub tcp: Counts<IntCounter>,
        pub udp: Counts<IntCounter>,
    }

    impl Default for Metrics {
        fn default() -> Self {
            Self::new().expect("Unable to initialize Prometheus metrics data!")
        }
    }

    impl Metrics {
        pub fn new() -> Result<Self> {
            Ok(Self {
                total: Counts::new("total")?,
                tcp: Counts::new("tcp")?,
                udp: Counts::new("udp")?,
                allocated: register_int_gauge!("allocated", "The number of allocated ports, count = 16383")?,
            })
        }

        /// # Example
        ///
        /// ```
        /// use turn_server::statistics::{prometheus::*, *};
        /// use turn_server::stun::Transport;
        ///
        /// METRICS.add(Transport::TCP, &Stats::ReceivedBytes(1));
        /// assert_eq!(METRICS.tcp.received_bytes.get(), 1);
        /// assert_eq!(METRICS.total.received_bytes.get(), 1);
        /// assert_eq!(METRICS.udp.received_bytes.get(), 0);
        /// ```
        pub fn add(&self, transport: Transport, payload: &Stats) {
            self.total.add(payload);

            if transport == Transport::TCP {
                self.tcp.add(payload);
            } else {
                self.udp.add(payload);
            }
        }
    }

    /// Generate prometheus metrics data that externally needs to be exposed to
    /// the `/metrics` route.
    pub fn generate_metrics(buf: &mut Vec<u8>) -> Result<()> {
        TextEncoder::new().encode(&prometheus::gather(), buf)?;
        Ok(())
    }
}

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
pub struct Statistics(Arc<RwLock<AHashMap<SessionAddr, Counts<Count>>>>);

impl Default for Statistics {
    #[cfg(feature = "api")]
    fn default() -> Self {
        Self(Arc::new(RwLock::new(AHashMap::with_capacity(1024))))
    }

    // There's no need to take up so much memory when you don't have stats enabled.
    #[cfg(not(feature = "api"))]
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
    /// use turn_server::stun::Transport;
    /// use turn_server::turn::*;
    ///
    /// let statistics = Statistics::default();
    /// let sender = statistics.get_reporter(Transport::UDP);
    ///
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// sender.send(&addr, &[Stats::ReceivedBytes(100)]);
    /// ```
    pub fn get_reporter(&self, transport: Transport) -> StatisticsReporter {
        StatisticsReporter {
            table: self.0.clone(),
            transport,
        }
    }

    /// Add an address to the watch list
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn_server::statistics::*;
    /// use turn_server::turn::*;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// statistics.register(addr.clone());
    /// assert_eq!(statistics.get(&addr).is_some(), true);
    /// ```
    pub fn register(&self, addr: SessionAddr) {
        #[cfg(feature = "prometheus")]
        {
            self::prometheus::METRICS.allocated.inc();
        }

        self.0.write().insert(
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
    /// use turn_server::turn::*;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// statistics.register(addr.clone());
    /// assert_eq!(statistics.get(&addr).is_some(), true);
    ///
    /// statistics.unregister(&addr);
    /// assert_eq!(statistics.get(&addr).is_some(), false);
    /// ```
    pub fn unregister(&self, addr: &SessionAddr) {
        #[cfg(feature = "prometheus")]
        {
            self::prometheus::METRICS.allocated.dec();
        }

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
    /// use turn_server::turn::*;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// statistics.register(addr.clone());
    /// assert_eq!(statistics.get(&addr).is_some(), true);
    /// ```
    pub fn get(&self, addr: &SessionAddr) -> Option<Counts<usize>> {
        self.0.read().get(addr).map(|counts| Counts {
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
    table: Arc<RwLock<AHashMap<SessionAddr, Counts<Count>>>>,
    transport: Transport,
}

impl StatisticsReporter {
    #[allow(unused_variables)]
    pub fn send(&self, addr: &SessionAddr, reports: &[Stats]) {
        #[cfg(feature = "api")]
        {
            #[cfg(feature = "prometheus")]
            {
                for report in reports {
                    self::prometheus::METRICS.add(self.transport, report);
                }
            }

            if let Some(counts) = self.table.read().get(addr) {
                for item in reports {
                    counts.add(item);
                }
            }
        }
    }
}
