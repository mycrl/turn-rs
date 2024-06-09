pub mod api;
pub mod config;
pub mod observer;
pub mod router;
pub mod server;
pub mod statistics;

use std::sync::Arc;

use turn::Service;

use self::{config::Config, observer::Observer, statistics::Statistics};

/// In order to let the integration test directly use the turn-server crate and
/// start the server, a function is opened to replace the main function to
/// directly start the server.
pub async fn server_main(config: Arc<Config>) -> anyhow::Result<()> {
    let statistics = Statistics::default();
    let observer = Observer::new(config.clone(), statistics.clone()).await?;
    let externals = config.turn.get_externals();
    let service = Service::new(config.turn.realm.clone(), externals, observer);
    server::run(config.clone(), statistics.clone(), &service).await?;
    api::start_server(config, service, statistics).await?;
    Ok(())
}
