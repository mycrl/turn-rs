#[cfg(feature = "rpc")]
pub mod rpc;

pub mod config;
pub mod handler;
pub mod server;
pub mod statistics;

use std::sync::Arc;

use self::{config::Config, handler::Handler, statistics::Statistics};

use service::{Service, ServiceOptions};

#[rustfmt::skip]
static SOFTWARE: &str = concat!(
    "turn-rs.",
    env!("CARGO_PKG_VERSION")
);

/// In order to let the integration test directly use the turn-server crate and
/// start the server, a function is opened to replace the main function to
/// directly start the server.
pub async fn startup(config: Arc<Config>) -> anyhow::Result<()> {
    let statistics = Statistics::default();
    let service = Service::new(ServiceOptions {
        software: SOFTWARE.to_string(),
        realm: config.turn.realm.clone(),
        port_range: config.runtime.port_range,
        interfaces: config.turn.get_externals(),
        handler: Handler::new(config.clone(), statistics.clone()).await?,
    });

    server::start(&config, &statistics, &service).await?;

    #[cfg(feature = "rpc")]
    {
        rpc::start_server(config, service, statistics).await?;
    }

    // The turn server is non-blocking after it runs and needs to be kept from
    // exiting immediately if the api server is not enabled.
    #[cfg(not(feature = "rpc"))]
    {
        std::future::pending::<()>().await;
    }

    Ok(())
}
