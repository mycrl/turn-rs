pub mod provider;

mod buffer;
mod switch;

use anyhow::Result;
use tokio::task::JoinSet;

use self::switch::Switch;
use crate::{
    Service,
    config::{Config, Interface},
    server::provider::{ProviderServer, ServerOptions, tcp::TcpServer, udp::UdpServer},
    service::Transport,
    statistics::Statistics,
};

pub async fn start_server(config: Config, service: Service, statistics: Statistics) -> Result<()> {
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
