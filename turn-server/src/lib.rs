#[cfg(feature = "api")]
pub mod api;

pub mod config;
pub mod observer;
pub mod server;
pub mod statistics;

use std::sync::Arc;

use self::{config::Config, observer::Observer, statistics::Statistics};

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
        interfaces: config.turn.get_externals(),
        port_range: config.runtime.port_range.clone(),
        handler: Observer::new(config.clone(), statistics.clone()).await?,
    });

    server::start(&config, &statistics, &service).await?;

    #[cfg(feature = "api")]
    {
        api::start_server(config, service, statistics).await?;
    }

    // The turn server is non-blocking after it runs and needs to be kept from
    // exiting immediately if the api server is not enabled.
    #[cfg(not(feature = "api"))]
    {
        std::future::pending::<()>().await;
    }

    Ok(())
}
