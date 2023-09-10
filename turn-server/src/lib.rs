pub mod api;
pub mod config;
pub mod monitor;
pub mod observer;
pub mod router;
pub mod server;

use std::sync::Arc;

use api::controller::Controller;
use config::Config;
use monitor::Monitor;
use observer::Observer;
use turn_rs::Service;

/// In order to let the integration test directly use the turn-server crate and
/// start the server, a function is opened to replace the main function to
/// directly start the server.
pub async fn server_main(config: Arc<Config>) -> anyhow::Result<()> {
    let monitor = Monitor::default();
    let observer = Observer::new(config.clone(), monitor.clone());
    let externals = config.turn.get_externals();
    let service = Service::new(config.turn.realm.clone(), externals, observer);
    server::run(config.clone(), monitor.clone(), &service).await?;

    let ctr = Controller::new(config.clone(), monitor, service);
    api::start_controller_service(&config, &ctr).await?;
    Ok(())
}
