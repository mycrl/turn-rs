pub mod transport;

use std::{net::SocketAddr, sync::Arc};

use ahash::{HashMap, HashMapExt};
use anyhow::Result;
use bytes::Bytes;
use parking_lot::RwLock;

use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    task::JoinSet,
};

use crate::{
    Service,
    codec::message::methods::Method,
    config::{Config, Interface},
    server::transport::{Server, ServerOptions, Transport, tcp::TcpServer, udp::UdpServer},
    statistics::Statistics,
};

pub async fn start_server(config: Config, service: Service, statistics: Statistics) -> Result<()> {
    let exchanger = Exchanger::default();

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
                    exchanger.clone(),
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
                    exchanger.clone(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PayloadType {
    Message(Method),
    ChannelData,
}

/// Handles packet forwarding between transport protocols.
#[derive(Clone)]
pub struct Exchanger(Arc<RwLock<HashMap<SocketAddr, UnboundedSender<(Bytes, PayloadType)>>>>);

impl Default for Exchanger {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(HashMap::with_capacity(1024))))
    }
}

impl Exchanger {
    /// Get the socket reader for the route.
    ///
    /// Each transport protocol is layered according to its own socket, and
    /// the data forwarded to this socket can be obtained by routing.
    fn get_receiver(&self, interface: SocketAddr) -> UnboundedReceiver<(Bytes, PayloadType)> {
        let (sender, receiver) = unbounded_channel();
        self.0.write().insert(interface, sender);

        receiver
    }

    /// Send data to dispatcher.
    ///
    /// By specifying the socket identifier and destination address, the route
    /// is forwarded to the corresponding socket. However, it should be noted
    /// that calling this function will not notify whether the socket exists.
    /// If it does not exist, the data will be discarded by default.
    fn send(&self, interface: &SocketAddr, ty: PayloadType, data: Bytes) {
        let mut is_destroy = false;

        {
            if let Some(sender) = self.0.read().get(interface)
                && sender.send((data, ty)).is_err()
            {
                is_destroy = true;
            }
        }

        if is_destroy {
            self.remove(interface);
        }
    }

    /// delete socket.
    pub fn remove(&self, interface: &SocketAddr) {
        drop(self.0.write().remove(interface))
    }
}
