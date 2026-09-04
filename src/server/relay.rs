use std::net::SocketAddr;

use ahash::HashMap;
use anyhow::Result;
use parking_lot::Mutex;
use tokio::task::JoinHandle;

use crate::{
    Service,
    config::Interface,
    server::{
        provider::{ProviderServer, ServerOptions, tcp::TcpServer, udp::UdpServer},
        switch::Switch,
    },
    service::Transport,
    statistics::Statistics,
};

/// Relay server manager.
///
/// This manager is responsible for creating and closing relay servers.
pub struct RelayServerManager {
    switch: Switch,
    service: Service,
    statistics: Statistics,
    tasks: Mutex<HashMap<u16, JoinHandle<Result<()>>>>,
}

impl RelayServerManager {
    pub fn new(switch: Switch, service: Service, statistics: Statistics) -> Self {
        Self {
            switch,
            service,
            statistics,
            tasks: Default::default(),
        }
    }

    /// Create a relay server.
    ///
    /// The relay server is created when a new port is allocated.
    pub async fn create(&self, interface: Interface, port: u16) -> Result<()> {
        let options = ServerOptions {
            transport: interface.transport,
            idle_timeout: interface.idle_timeout,
            external: interface.external,
            listen: SocketAddr::new(interface.listen.ip(), port),
        };

        self.tasks.lock().insert(
            port,
            match interface.transport {
                Transport::Udp => tokio::spawn(UdpServer::bind(&options).await?.start(
                    options,
                    self.service.clone(),
                    self.statistics.clone(),
                    self.switch.clone(),
                )),
                Transport::Tcp => tokio::spawn(TcpServer::bind(&options).await?.start(
                    options,
                    self.service.clone(),
                    self.statistics.clone(),
                    self.switch.clone(),
                )),
            },
        );

        Ok(())
    }

    /// Close a relay server.
    ///
    /// The relay server is closed when the port is deallocated.
    pub fn close(&self, port: u16) {
        if let Some(handle) = self.tasks.lock().remove(&port) {
            handle.abort();
        }
    }
}
