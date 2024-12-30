use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use ahash::AHashMap;
use parking_lot::RwLock;
use stun::Transport;
use turn::Socket;

/// [issue](https://github.com/mycrl/turn-rs/issues/101)
///
/// Integrated Prometheus Metrics Exporter
pub mod prometheus {
    use anyhow::Result;
    use once_cell::sync::Lazy;
    use prometheus::{
        register_int_counter, register_int_gauge, Encoder, IntCounter, IntGauge, TextEncoder,
    };

    use super::{Counts, Number, Stats};

    use stun::Transport;

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

    pub static METRICS: Lazy<Metrics> = Lazy::new(|| Metrics::default());

    /// # Example
    ///
    /// ```
    /// use prometheus::register_int_counter;
    /// use turn_server::statistics::{prometheus::*, Number};
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
        fn add(&self, value: u64) {
            self.inc_by(value);
        }

        fn get(&self) -> u64 {
            self.get()
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
                allocated: register_int_gauge!(
                    "allocated",
                    "The number of allocated ports, count = 16383"
                )?,
            })
        }

        /// # Example
        ///
        /// ```
        /// use stun::Transport;
        /// use turn_server::statistics::{prometheus::*, *};
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
    ReceivedBytes(u32),
    SendBytes(u32),
    ReceivedPkts(u32),
    SendPkts(u32),
    ErrorPkts(u32),
}

pub trait Number {
    fn add(&self, value: u64);
    fn get(&self) -> u64;
}

#[derive(Default)]
pub struct Count(AtomicU64);

impl Number for Count {
    fn add(&self, value: u64) {
        self.0.fetch_add(value, Ordering::Relaxed);
    }

    fn get(&self) -> u64 {
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
            Stats::ReceivedBytes(v) => self.received_bytes.add(*v as u64),
            Stats::ReceivedPkts(v) => self.received_pkts.add(*v as u64),
            Stats::SendBytes(v) => self.send_bytes.add(*v as u64),
            Stats::SendPkts(v) => self.send_pkts.add(*v as u64),
            Stats::ErrorPkts(v) => self.error_pkts.add(*v as u64),
        }
    }
}

/// worker cluster statistics
#[derive(Clone)]
pub struct Statistics(Arc<RwLock<AHashMap<Socket, Counts<Count>>>>);

impl Default for Statistics {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(AHashMap::with_capacity(1024))))
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
    /// use stun::Transport;
    /// use turn::*;
    /// use turn_server::statistics::*;
    ///
    /// let statistics = Statistics::default();
    /// let sender = statistics.get_reporter(Transport::UDP);
    ///
    /// let socket = Socket {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// sender.send(&socket, &[Stats::ReceivedBytes(100)]);
    /// ```
    pub fn get_reporter(&self, transport: Transport) -> StatisticsReporter {
        StatisticsReporter {
            map: self.0.clone(),
            transport,
        }
    }

    /// Add an address to the watch list
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::*;
    /// use turn_server::statistics::*;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let socket = Socket {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// statistics.register(socket.clone());
    /// assert_eq!(statistics.get(&socket).is_some(), true);
    /// ```
    pub fn register(&self, socket: Socket) {
        #[cfg(feature = "prometheus")]
        {
            self::prometheus::METRICS.allocated.inc();
        }

        self.0.write().insert(
            socket,
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
    /// use turn::*;
    /// use turn_server::statistics::*;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let socket = Socket {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// statistics.register(socket.clone());
    /// assert_eq!(statistics.get(&socket).is_some(), true);
    ///
    /// statistics.unregister(&socket);
    /// assert_eq!(statistics.get(&socket).is_some(), false);
    /// ```
    pub fn unregister(&self, socket: &Socket) {
        #[cfg(feature = "prometheus")]
        {
            self::prometheus::METRICS.allocated.dec();
        }

        self.0.write().remove(socket);
    }

    /// Obtain a list of statistics from statisticsing
    ///
    /// The obtained list is in the same order as it was added.
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use turn::*;
    /// use turn_server::statistics::*;
    ///
    /// let statistics = Statistics::default();
    ///
    /// let socket = Socket {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// statistics.register(socket.clone());
    /// assert_eq!(statistics.get(&socket).is_some(), true);
    /// ```
    pub fn get(&self, socket: &Socket) -> Option<Counts<u64>> {
        self.0.read().get(socket).map(|counts| Counts {
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
pub struct StatisticsReporter {
    #[allow(unused)]
    transport: Transport,
    map: Arc<RwLock<AHashMap<Socket, Counts<Count>>>>,
}

impl StatisticsReporter {
    #[allow(unused_variables)]
    pub fn send(&self, socket: &Socket, reports: &[Stats]) {
        #[cfg(feature = "prometheus")]
        {
            for report in reports {
                self::prometheus::METRICS.add(self.transport, report);
            }
        }

        if let Some(counts) = self.map.read().get(socket) {
            for item in reports {
                counts.add(item);
            }
        }
    }
}
