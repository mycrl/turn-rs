#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "prometheus")]
pub mod prometheus;

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

use self::{
    config::Config,
    handler::Handler,
    service::{ServiceHandler, ServiceOptions},
    statistics::Statistics,
};

use tokio::{
    sync::{oneshot, watch},
    task::{JoinHandle, JoinSet},
};

#[rustfmt::skip]
pub(crate) static SOFTWARE: &str = concat!(
    "turn-rs.",
    env!("CARGO_PKG_VERSION")
);

pub(crate) type Service = service::Service<Handler>;

/// In order to let the integration test directly use the turn-server crate and
/// start the server, a function is opened to replace the main function to
/// directly start the server.
#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::{Config, spawn_server, spawn_server_with_handler};
    use crate::{
        codec::{crypto::Password, message::attributes::PasswordAlgorithm},
        config::{Interface, Server},
        service::{ServiceHandler, session::Identifier},
    };

    #[derive(Clone)]
    struct AllowAllHandler;

    impl ServiceHandler for AllowAllHandler {
        async fn get_password(
            &self,
            _id: &Identifier,
            _username: &str,
            _algorithm: PasswordAlgorithm,
        ) -> Option<Password> {
            None
        }
    }

    fn test_config() -> Config {
        Config {
            server: Server {
                interfaces: vec![Interface::Udp {
                    listen: "127.0.0.1:0"
                        .parse::<SocketAddr>()
                        .expect("test listen address should parse"),
                    external: "127.0.0.1:0"
                        .parse::<SocketAddr>()
                        .expect("test external address should parse"),
                    idle_timeout: 1,
                    mtu: 1500,
                }],
                ..Server::default()
            },
            ..Config::default()
        }
    }

    #[tokio::test]
    async fn spawned_server_stops_cleanly() {
        spawn_server(test_config())
            .await
            .expect("server should start")
            .shutdown()
            .await
            .expect("server should stop cleanly");
    }

    #[tokio::test]
    async fn custom_handler_server_stops_cleanly() {
        spawn_server_with_handler(test_config(), AllowAllHandler)
            .await
            .expect("server should start")
            .shutdown()
            .await
            .expect("server should stop cleanly");
    }

    #[tokio::test]
    async fn shutdown_releases_listener_port() {
        let reserved_socket =
            std::net::UdpSocket::bind("127.0.0.1:0").expect("test socket should bind");
        let address = reserved_socket
            .local_addr()
            .expect("test socket should have a local address");
        drop(reserved_socket);

        let mut config = test_config();
        config.server.interfaces = vec![Interface::Udp {
            listen: address,
            external: address,
            idle_timeout: 1,
            mtu: 1500,
        }];

        spawn_server_with_handler(config, AllowAllHandler)
            .await
            .expect("server should bind the reserved port")
            .shutdown()
            .await
            .expect("server should stop cleanly");

        std::net::UdpSocket::bind(address).expect("shutdown should release the listener port");
    }

    #[tokio::test]
    async fn spawn_reports_listener_bind_failure() {
        let occupied_socket =
            std::net::UdpSocket::bind("127.0.0.1:0").expect("test socket should bind");
        let mut config = test_config();
        let occupied_address = occupied_socket
            .local_addr()
            .expect("test socket should have a local address");
        config.server.interfaces = vec![Interface::Udp {
            listen: occupied_address,
            external: occupied_address,
            idle_timeout: 1,
            mtu: 1500,
        }];

        let error = match spawn_server_with_handler(config, AllowAllHandler).await {
            Ok(_) => panic!("spawn should return the listener bind failure"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("Address already in use"));
    }
}

/// Owns a spawned TURN server and provides graceful shutdown.
pub struct ServerHandle {
    shutdown: watch::Sender<bool>,
    task: JoinHandle<anyhow::Result<()>>,
}

impl ServerHandle {
    /// Stops the TURN server and waits for its listener tasks to exit.
    pub async fn shutdown(self) -> anyhow::Result<()> {
        let _ = self.shutdown.send(true);
        self.task.await??;
        Ok(())
    }
}

async fn build_service(config: &Config, statistics: Statistics) -> anyhow::Result<Service> {
    Ok(service::Service::new(ServiceOptions {
        realm: config.server.realm.clone(),
        port_range: config.server.port_range,
        interfaces: config.server.get_interface_addrs(),
        handler: Handler::new(config.clone(), statistics).await?,
    }))
}

async fn run_server(
    config: Config,
    service: Service,
    statistics: Statistics,
    shutdown: watch::Receiver<bool>,
    startup: Option<oneshot::Sender<anyhow::Result<()>>>,
) -> anyhow::Result<()> {
    let mut workers = JoinSet::new();

    workers.spawn(server::start_server(
        config.clone(),
        service.clone(),
        statistics.clone(),
        shutdown,
        startup,
    ));

    #[cfg(feature = "prometheus")]
    workers.spawn(prometheus::start_server(config.clone()));

    #[cfg(feature = "api")]
    workers.spawn(api::start_server(config, service, statistics));

    if let Some(result) = workers.join_next().await {
        workers.abort_all();
        return result?;
    }

    Ok(())
}

/// Starts a TURN server in a background task and returns its lifecycle handle.
pub async fn spawn_server(config: Config) -> anyhow::Result<ServerHandle> {
    let statistics = Statistics::default();
    let service = build_service(&config, statistics.clone()).await?;
    let (shutdown, shutdown_rx) = watch::channel(false);
    let (startup_sender, startup_receiver) = oneshot::channel();
    let task = tokio::spawn(run_server(
        config,
        service,
        statistics,
        shutdown_rx,
        Some(startup_sender),
    ));

    match startup_receiver.await {
        Ok(Ok(())) => Ok(ServerHandle { shutdown, task }),
        Ok(Err(error)) => {
            let _ = task.await;
            Err(error)
        }
        Err(_) => {
            let result = task.await?;
            Err(result
                .err()
                .unwrap_or_else(|| anyhow::anyhow!("TURN server exited before startup")))
        }
    }
}

/// Starts a TURN server with an application-provided authentication and peer-policy handler.
///
/// This entry point runs only the TURN transport runtime; callers own any management
/// API and metrics services they need alongside it.
pub async fn spawn_server_with_handler<T>(
    config: Config,
    handler: T,
) -> anyhow::Result<ServerHandle>
where
    T: ServiceHandler + Clone,
{
    let statistics = Statistics::default();
    let service = service::Service::new(ServiceOptions {
        realm: config.server.realm.clone(),
        port_range: config.server.port_range,
        interfaces: config.server.get_interface_addrs(),
        handler,
    });
    let (shutdown, shutdown_rx) = watch::channel(false);
    let (startup_sender, startup_receiver) = oneshot::channel();
    let task = tokio::spawn(server::start_server(
        config,
        service,
        statistics,
        shutdown_rx,
        Some(startup_sender),
    ));

    match startup_receiver.await {
        Ok(Ok(())) => Ok(ServerHandle { shutdown, task }),
        Ok(Err(error)) => {
            let _ = task.await;
            Err(error)
        }
        Err(_) => {
            let result = task.await?;
            Err(result
                .err()
                .unwrap_or_else(|| anyhow::anyhow!("TURN server exited before startup")))
        }
    }
}

/// Starts a TURN server and waits until it exits.
pub async fn start_server(config: Config) -> anyhow::Result<()> {
    let statistics = Statistics::default();
    let service = build_service(&config, statistics.clone()).await?;
    let (_shutdown, shutdown_rx) = watch::channel(false);
    run_server(config, service, statistics, shutdown_rx, None).await
}
