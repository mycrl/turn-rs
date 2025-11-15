#[cfg(feature = "rpc")]
pub mod rpc;

pub mod codec;
pub mod config;
pub mod handler;
pub mod server;
pub mod service;
pub mod statistics;

pub mod prelude {
    pub use super::codec::{
        channel_data::*,
        crypto::*,
        message::{
            attributes::{error::*, *},
            methods::*,
            *,
        },
        *,
    };

    pub use super::service::{
        session::{ports::*, *},
        *,
    };
}

use self::{config::Config, handler::Handler, service::ServiceOptions, statistics::Statistics};

use tokio::task::JoinSet;

#[rustfmt::skip]
pub(crate) static SOFTWARE: &str = concat!(
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
        realm: config.server.realm.clone(),
        port_range: config.server.port_range,
        interfaces: config.server.get_externals(),
        handler: Handler::new(config.clone(), statistics.clone()).await?,
    });

    {
        let mut workers = JoinSet::new();

        workers.spawn(server::start_server(
            config.clone(),
            service.clone(),
            statistics.clone(),
        ));

        #[cfg(feature = "rpc")]
        workers.spawn(rpc::start_server(config, service, statistics));

        if let Some(res) = workers.join_next().await {
            workers.abort_all();

            return res?;
        }
    }

    Ok(())
}
