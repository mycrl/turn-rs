pub mod provider;

mod buffer;
mod switch;

use anyhow::Result;
use tokio::{
    sync::{mpsc::unbounded_channel, oneshot, watch},
    task::JoinSet,
};

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
    startup: Option<oneshot::Sender<Result<()>>>,
) -> Result<()>
where
    T: ServiceHandler + Clone,
{
    let switch = Switch::default();

    let mut servers = JoinSet::new();
    let interface_count = config.server.interfaces.len();
    let (startup_sender, mut startup_receiver) = unbounded_channel();

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
                    startup_sender.clone(),
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
                    startup_sender.clone(),
                ));
            }
        };
    }

    drop(startup_sender);
    let startup_result = async {
        for _ in 0..interface_count {
            startup_receiver.recv().await.ok_or_else(|| {
                anyhow::anyhow!("TURN listener exited before reporting startup")
            })??;
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;

    if let Some(startup) = startup {
        let _ = startup.send(
            startup_result
                .as_ref()
                .map(|_| ())
                .map_err(|error| anyhow::anyhow!(error.to_string())),
        );
    }

    if let Err(error) = startup_result {
        servers.abort_all();
        return Err(error);
    }

    // As soon as one server exits, all servers will be exited to ensure the
    // availability of all servers.
    if let Some(res) = servers.join_next().await {
        servers.abort_all();

        return res?;
    }

    Ok(())
}
