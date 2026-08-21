pub mod provider;

mod buffer;
mod switch;

use anyhow::Result;
use tokio::task::JoinSet;

use self::switch::Switch;
use crate::{
    Service,
    config::Config,
    server::provider::{ProviderServer, ServerOptions, tcp::TcpServer, udp::UdpServer},
    service::Transport,
    statistics::Statistics,
};

pub async fn start_server(config: Config, service: Service, statistics: Statistics) -> Result<()> {
    let switch = Switch::default();

    let mut servers = JoinSet::new();

    for interface in config.server.interfaces {
        let options = ServerOptions {
            transport: interface.transport,
            idle_timeout: interface.idle_timeout,
            external: interface.external,
            listen: interface.listen,
        };

        match interface.transport {
            Transport::Udp => {
                servers.spawn(UdpServer::start(
                    options,
                    service.clone(),
                    statistics.clone(),
                    switch.clone(),
                ));
            }
            Transport::Tcp => {
                servers.spawn(TcpServer::start(
                    options,
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
