use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use ahash::HashMap;
use parking_lot::RwLock;

use crate::service::{Transport, session::Identifier};

#[cfg(feature = "prometheus")]
use anyhow::Result;

#[cfg(feature = "prometheus")]
use prometheus::{Encoder, IntCounter, IntGauge, Registry, TextEncoder};

/// The type of information passed in the statistics channel
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

#[cfg(feature = "prometheus")]
impl Number for IntCounter {
    fn add(&self, value: usize) {
        self.inc_by(value as u64);
    }

    fn get(&self) -> usize {
        IntCounter::get(self) as usize
    }
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

/// Worker independent statistics
pub struct Counts<T> {
    pub received_bytes: T,
    pub send_bytes: T,
    pub received_pkts: T,
    pub send_pkts: T,
    pub error_pkts: T,
}

#[cfg(feature = "prometheus")]
impl Counts<IntCounter> {
    fn new(prefix: &str, registry: &Registry) -> Result<Self> {
        fn counter(
            registry: &Registry,
            prefix: &str,
            operation: &str,
            dst: &str,
        ) -> Result<IntCounter> {
            let counter = IntCounter::new(
                format!("{prefix}_{operation}_{dst}"),
                format!("The {prefix} amount of {dst} {operation}"),
            )?;

            registry.register(Box::new(counter.clone()))?;

            Ok(counter)
        }

        Ok(Self {
            received_bytes: counter(registry, prefix, "received", "bytes")?,
            send_bytes: counter(registry, prefix, "sent", "bytes")?,
            received_pkts: counter(registry, prefix, "received", "packets")?,
            send_pkts: counter(registry, prefix, "sent", "packets")?,
            error_pkts: counter(registry, prefix, "error", "packets")?,
        })
    }
}

#[cfg(feature = "prometheus")]
struct Metrics {
    registry: Registry,
    allocated: IntGauge,
    total: Counts<IntCounter>,
    tcp: Counts<IntCounter>,
    udp: Counts<IntCounter>,
}

#[cfg(feature = "prometheus")]
impl Metrics {
    fn new() -> Result<Self> {
        let registry = Registry::new();
        let allocated = IntGauge::new("allocated", "The number of allocated ports")?;
        registry.register(Box::new(allocated.clone()))?;

        Ok(Self {
            total: Counts::new("total", &registry)?,
            tcp: Counts::new("tcp", &registry)?,
            udp: Counts::new("udp", &registry)?,
            allocated,
            registry,
        })
    }

    fn add(&self, transport: Transport, payload: &Stats) {
        self.total.add(payload);

        match transport {
            Transport::Tcp => self.tcp.add(payload),
            Transport::Udp => self.udp.add(payload),
        }
    }

    fn encode(&self, buf: &mut Vec<u8>) -> Result<()> {
        TextEncoder::new().encode(&self.registry.gather(), buf)?;

        Ok(())
    }
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
pub struct Statistics {
    table: Arc<RwLock<HashMap<Identifier, Counts<Count>>>>,
    #[cfg(feature = "prometheus")]
    metrics: Arc<Metrics>,
}

impl Default for Statistics {
    #[cfg(feature = "rpc")]
    fn default() -> Self {
        use ahash::HashMapExt;

        Self {
            table: Arc::new(RwLock::new(HashMap::with_capacity(1024))),
            #[cfg(feature = "prometheus")]
            metrics: Arc::new(
                Metrics::new().expect("Unable to initialize Prometheus metrics data!"),
            ),
        }
    }

    // There's no need to take up so much memory when you don't have stats enabled.
    #[cfg(not(feature = "rpc"))]
    fn default() -> Self {
        Self {
            table: Default::default(),
        }
    }
}

impl Statistics {
    /// get signal sender
    ///
    /// The signal sender can notify the statistics instance to update
    /// internal statistics.
    ///
    /// # Example
    ///
    /// ```
    /// use turn_server::statistics::*;
    /// use turn_server::service::session::Identifier;
    /// use turn_server::service::Transport;
    ///
    /// let statistics = Statistics::default();
    /// let sender = statistics.get_reporter(Transport::Tcp);
    ///
    /// let identifier = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     external: "127.0.0.1:3478".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::Tcp,
    /// };
    ///
    /// sender.send(&identifier, &[Stats::ReceivedBytes(100)]);
    /// ```
    pub fn get_reporter(&self, transport: Transport) -> StatisticsReporter {
        StatisticsReporter {
            table: self.table.clone(),
            transport,
            #[cfg(feature = "prometheus")]
            metrics: self.metrics.clone(),
        }
    }

    /// Add an address to the watch list
    ///
    /// # Example
    ///
    /// ```
    /// use turn_server::statistics::*;
    /// use turn_server::service::session::Identifier;
    /// use turn_server::service::Transport;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let identifier = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     external: "127.0.0.1:3478".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::Udp,
    /// };
    ///
    /// statistics.register(identifier.clone());
    /// assert_eq!(statistics.get(&identifier).is_some(), true);
    /// ```
    pub fn register(&self, identifier: Identifier) {
        #[cfg(feature = "prometheus")]
        self.metrics.allocated.inc();

        self.table.write().insert(
            identifier,
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
    /// use turn_server::statistics::*;
    /// use turn_server::service::session::Identifier;
    /// use turn_server::service::Transport;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let identifier = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     external: "127.0.0.1:3478".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::Udp,
    /// };
    ///
    /// statistics.register(identifier.clone());
    /// assert_eq!(statistics.get(&identifier).is_some(), true);
    ///
    /// statistics.unregister(&identifier);
    /// assert_eq!(statistics.get(&identifier).is_some(), false);
    /// ```
    pub fn unregister(&self, identifier: &Identifier) {
        #[cfg(feature = "prometheus")]
        self.metrics.allocated.dec();

        self.table.write().remove(identifier);
    }

    /// Obtain a list of statistics from statistics
    ///
    /// The obtained list is in the same order as it was added.
    ///
    /// # Example
    ///
    /// ```
    /// use turn_server::statistics::*;
    /// use turn_server::service::session::Identifier;
    /// use turn_server::service::Transport;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let identifier = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     external: "127.0.0.1:3478".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::Udp,
    /// };
    ///
    /// statistics.register(identifier.clone());
    /// assert_eq!(statistics.get(&identifier).is_some(), true);
    /// ```
    pub fn get(&self, identifier: &Identifier) -> Option<Counts<usize>> {
        self.table.read().get(identifier).map(|counts| Counts {
            received_bytes: counts.received_bytes.get(),
            received_pkts: counts.received_pkts.get(),
            send_bytes: counts.send_bytes.get(),
            send_pkts: counts.send_pkts.get(),
            error_pkts: counts.error_pkts.get(),
        })
    }

    #[cfg(feature = "prometheus")]
    pub fn encode_prometheus(&self, buf: &mut Vec<u8>) -> Result<()> {
        self.metrics.encode(buf)
    }
}

/// statistics reporter
///
/// It is held by each worker, and status information can be sent to the
/// statistics instance through this instance to update the internal
/// statistical information of the statistics.
#[derive(Clone)]
#[allow(unused)]
pub struct StatisticsReporter {
    table: Arc<RwLock<HashMap<Identifier, Counts<Count>>>>,
    transport: Transport,
    #[cfg(feature = "prometheus")]
    metrics: Arc<Metrics>,
}

impl StatisticsReporter {
    #[allow(unused_variables)]
    pub fn send(&self, identifier: &Identifier, reports: &[Stats]) {
        #[cfg(feature = "rpc")]
        {
            #[cfg(feature = "prometheus")]
            {
                for report in reports {
                    self.metrics.add(self.transport, report);
                }
            }

            if let Some(counts) = self.table.read().get(identifier) {
                for item in reports {
                    counts.add(item);
                }
            }
        }
    }
}
