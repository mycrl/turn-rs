use super::{Counts, Number, Stats};

use std::sync::LazyLock;

use anyhow::Result;
use codec::message::attributes::Transport;
use prometheus::{
    Encoder, IntCounter, IntGauge, TextEncoder, register_int_counter, register_int_gauge,
};

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
            allocated: register_int_gauge!(
                "allocated",
                "The number of allocated ports, count = 16383"
            )?,
        })
    }

    /// # Example
    ///
    /// ```
    /// use turn_server::statistics::{prometheus::*, *};
    /// use codec::message::attributes::Transport;
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
