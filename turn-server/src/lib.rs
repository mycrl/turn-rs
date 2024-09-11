pub mod config;
pub mod credentials;
pub mod observer;
pub mod publicly;
pub mod router;
pub mod server;
pub mod statistics;

use std::sync::Arc;

use turn::Service;

use self::{
    config::Config, credentials::StaticCredentials, observer::Observer, statistics::Statistics,
};

/// In order to let the integration test directly use the turn-server crate and
/// start the server, a function is opened to replace the main function to
/// directly start the server.
pub async fn startup(config: Arc<Config>) -> anyhow::Result<()> {
    let statistics = Statistics::default();
    let credentials = StaticCredentials::from(config.auth.clone());

    let service = Service::new(
        config.turn.realm.clone(),
        config.turn.get_externals(),
        Observer::new(config.clone(), statistics.clone(), credentials.clone()).await?,
    );

    server::run(config.clone(), statistics.clone(), &service).await?;
    publicly::start_server(config, service, statistics, credentials.clone()).await?;

    Ok(())
}
