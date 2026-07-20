mod hooks;
mod server;

pub use self::hooks::{HooksEvent, RpcHooksService};

use std::time::{Duration, Instant};

use self::server::RpcServer;
use crate::{Service, config::Config, statistics::Statistics};

use anyhow::Result;
use sdk::protos::turn_service_server::TurnServiceServer;
use tonic::transport::Server;

#[cfg(feature = "ssl")]
use tonic::transport::{Identity, ServerTlsConfig};

pub async fn start_server(config: Config, service: Service, statistics: Statistics) -> Result<()> {
    if let Some(api) = &config.api {
        let mut builder = Server::builder();

        builder = builder
            .timeout(Duration::from_secs(api.timeout as u64))
            .accept_http1(false);

        #[cfg(feature = "ssl")]
        if let Some(ssl) = &api.ssl {
            builder = builder.tls_config(ServerTlsConfig::new().identity(Identity::from_pem(
                ssl.certificate_chain.clone(),
                ssl.private_key.clone(),
            )))?;
        }

        log::info!("api server listening: listen={}", api.listen);

        builder
            .add_service(TurnServiceServer::new(RpcServer {
                config: config.clone(),
                uptime: Instant::now(),
                statistics,
                service,
            }))
            .serve(api.listen)
            .await?;
    } else {
        std::future::pending().await
    }

    Ok(())
}
