pub mod api;
pub mod config;
pub mod observer;
pub mod router;
pub mod server;
pub mod statistics;
pub mod stun;
pub mod turn;

use std::sync::Arc;

use self::{config::Config, observer::Observer, statistics::Statistics, turn::Service};

// Re-export ServerHandle for external use
pub use server::ServerHandle;

#[rustfmt::skip]
static SOFTWARE: &str = concat!(
    "turn-rs.",
    env!("CARGO_PKG_VERSION")
);

/// In order to let the integration test directly use the turn-server crate and
/// start the server, a function is opened to replace the main function to
/// directly start the server.
pub async fn startup(config: Arc<Config>) -> anyhow::Result<()> {
    let _server_handle = startup_with_handle(config).await?;

    // The turn server is non-blocking after it runs and needs to be kept from
    // exiting immediately if the api server is not enabled.
    #[cfg(not(feature = "api"))]
    {
        _server_handle.wait_for_shutdown().await?;
    }

    Ok(())
}

/// Start the server and return a handle for controlling it.
/// This allows external code to manage server lifecycle.
pub async fn startup_with_handle(config: Arc<Config>) -> anyhow::Result<ServerHandle> {
    let statistics = Statistics::default();
    let service = Service::new(
        SOFTWARE.to_string(),
        config.turn.realm.clone(),
        config.turn.get_externals(),
        Observer::new(config.clone(), statistics.clone()).await?,
    );

    let server_handle = server::start(&config, &statistics, &service).await?;

    #[cfg(feature = "api")]
    {
        api::start_server(config, service, statistics).await?;
    }

    Ok(server_handle)
}
