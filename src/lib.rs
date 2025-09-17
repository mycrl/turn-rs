#[cfg(feature = "rpc")]
pub mod rpc;

pub mod config;
pub mod handler;
pub mod server;
pub mod statistics;

use self::{config::Config, handler::Handler, statistics::Statistics};

use service::ServiceOptions;

#[rustfmt::skip]
static SOFTWARE: &str = concat!(
    "turn-rs.",
    env!("CARGO_PKG_VERSION")
);

pub(crate) type Service = service::Service<Handler>;

/// In order to let the integration test directly use the turn-server crate and
/// start the server, a function is opened to replace the main function to
/// directly start the server.
pub async fn start_server(config: Config) -> anyhow::Result<()> {
    let statistics = Statistics::default();
    let service = service::Service::new(ServiceOptions {
        software: SOFTWARE.to_string(),
        realm: config.turn.realm.clone(),
        port_range: config.runtime.port_range,
        interfaces: config.turn.get_externals(),
        handler: Handler::new(config.clone(), statistics.clone()).await?,
    });

    tokio::try_join!(
        server::start_server(config.clone(), service.clone(), statistics.clone()),
        #[cfg(feature = "rpc")]
        rpc::start_server(config, service, statistics),
    )?;

    Ok(())
}
