pub mod provider;

mod buffer;
mod switch;

use anyhow::Result;
use tokio::{sync::watch, task::JoinSet};

use self::switch::Switch;
use crate::{
    config::{Config, Interface},
    server::provider::{ProviderServer, ServerOptions, tcp::TcpServer, udp::UdpServer},
    service::{Service, ServiceHandler, Transport},
    statistics::Statistics,
};

pub async fn start_server<T>(
    config: Config,
    service: Service<T>,
    statistics: Statistics,
    shutdown: watch::Receiver<bool>,
) -> Result<()>
where
    T: ServiceHandler + Clone,
{
    let switch = Switch::default();

    let mut servers = JoinSet::new();

    for interface in config.server.interfaces {
        match interface {
            Interface::Udp {
                listen,
                external,
                idle_timeout,
                mtu,
            } => {
                servers.spawn(UdpServer::start(
                    ServerOptions {
                        transport: Transport::Udp,
                        idle_timeout,
                        ssl: None,
                        external,
                        listen,
                        mtu,
                    },
                    service.clone(),
                    statistics.clone(),
                    switch.clone(),
                    shutdown.clone(),
                ));
            }
            Interface::Tcp {
                listen,
                external,
                idle_timeout,
                ssl,
            } => {
                servers.spawn(TcpServer::start(
                    ServerOptions {
                        transport: Transport::Tcp,
                        idle_timeout,
                        external,
                        listen,
                        mtu: 0,
                        ssl,
                    },
                    service.clone(),
                    statistics.clone(),
                    switch.clone(),
                    shutdown.clone(),
                ));
            }
        };
    }

    // As soon as one server exits, all servers will be exited to ensure the
    // availability of all servers.
    if let Some(res) = servers.join_next().await {
        servers.abort_all();

        return res?;
    }

    Ok(())
}
