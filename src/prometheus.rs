use std::sync::LazyLock;

use anyhow::Result;
use axum::{
    Router,
    http::{StatusCode, header::CONTENT_TYPE},
    response::IntoResponse,
    routing::get,
};

use prometheus::{
    Encoder, IntCounter, IntGauge, TextEncoder, register_int_counter, register_int_gauge,
};

use tokio::net::TcpListener;

use crate::{
    config::Config,
    server::transport::Transport,
    statistics::{Counts, Number, Stats},
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

impl Number for IntCounter {
    fn add(&self, value: usize) {
        self.inc_by(value as u64);
    }

    fn get(&self) -> usize {
        IntCounter::get(self) as usize
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
    fn new() -> Result<Self> {
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

    pub fn add(&self, transport: Transport, payload: &Stats) {
        self.total.add(payload);

        if transport == Transport::Tcp {
            self.tcp.add(payload);
        } else {
            self.udp.add(payload);
        }
    }
}

/// Generate prometheus metrics data that externally needs to be exposed to
/// the `/metrics` route.
fn generate_metrics(buf: &mut Vec<u8>) -> Result<()> {
    TextEncoder::new().encode(&prometheus::gather(), buf)?;

    Ok(())
}

pub async fn start_server(config: Config) -> Result<()> {
    if let Some(config) = config.prometheus {
        let mut metrics_bytes = Vec::with_capacity(4096);

        let app = Router::new().route(
            "/metrics",
            get(|| async move {
                metrics_bytes.clear();

                if generate_metrics(&mut metrics_bytes).is_err() {
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                } else {
                    ([(CONTENT_TYPE, "text/plain")], metrics_bytes).into_response()
                }
            }),
        );

        #[cfg(feature = "ssl")]
        if let Some(ssl) = &config.ssl {
            axum_server::bind_rustls(
                config.listen,
                axum_server::tls_rustls::RustlsConfig::from_pem_chain_file(
                    ssl.certificate_chain.clone(),
                    ssl.private_key.clone(),
                )
                .await?,
            )
            .serve(app.into_make_service())
            .await?;

            log::info!("prometheus server listening={:?}", config.listen);

            return Ok(());
        }

        {
            let listener = TcpListener::bind(config.listen).await?;

            log::info!("prometheus server listening={:?}", config.listen);

            axum::serve(listener, app).await?;
        }
    } else {
        std::future::pending().await
    };

    Ok(())
}
