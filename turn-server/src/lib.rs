pub mod api;
pub mod config;
pub mod router;
pub mod server;

use std::{net::SocketAddr, sync::Arc};

use api::{controller::Controller, hooks::Hooks, payload::Events};
use async_trait::async_trait;
use config::Config;
use server::Monitor;
use turn_rs::{Observer, Service};

struct TObserver {
    hooks: Hooks,
    monitor: Monitor,
}

impl TObserver {
    fn new(cfg: Arc<Config>, monitor: Monitor) -> Self {
        Self {
            hooks: Hooks::new(cfg),
            monitor,
        }
    }
}

#[async_trait]
impl Observer for TObserver {
    async fn auth(&self, addr: &SocketAddr, name: &str) -> Option<String> {
        let pwd = self.hooks.auth(addr, name).await.ok();
        log::info!("auth: addr={:?}, name={:?}, pwd={:?}", addr, name, pwd);
        pwd
    }

    fn allocated(&self, addr: &SocketAddr, name: &str, port: u16) {
        log::info!("allocate: addr={:?}, name={:?}, port={}", addr, name, port);
        self.monitor.set(*addr);
        self.hooks
            .on_events(&Events::Allocated { addr, name, port });
    }

    fn binding(&self, addr: &SocketAddr) {
        log::info!("binding: addr={:?}", addr);
        self.hooks.on_events(&Events::Binding { addr });
    }

    fn channel_bind(&self, addr: &SocketAddr, name: &str, number: u16) {
        log::info!(
            "channel bind: addr={:?}, name={:?}, number={}",
            addr,
            name,
            number
        );

        self.hooks
            .on_events(&Events::ChannelBind { addr, name, number });
    }

    fn create_permission(&self, addr: &SocketAddr, name: &str, relay: &SocketAddr) {
        log::info!(
            "create permission: addr={:?}, name={:?}, realy={:?}",
            addr,
            name,
            relay
        );

        self.hooks
            .on_events(&Events::CreatePermission { addr, name, relay });
    }

    fn refresh(&self, addr: &SocketAddr, name: &str, time: u32) {
        log::info!("refresh: addr={:?}, name={:?}, time={}", addr, name, time);
        self.hooks.on_events(&Events::Refresh { addr, name, time });
    }

    fn abort(&self, addr: &SocketAddr, name: &str) {
        log::info!("node abort: addr={:?}, name={:?}", addr, name);
        self.monitor.delete(addr);
        self.hooks.on_events(&Events::Abort { addr, name });
    }
}

/// In order to let the integration test directly use the turn-server crate and
/// start the server, a function is opened to replace the main function to
/// directly start the server.
pub async fn server_main(config: Arc<Config>) -> anyhow::Result<()> {
    let monitor = Monitor::new();
    let observer = TObserver::new(config.clone(), monitor.clone());
    let externals = config.turn.get_externals();
    let service = Service::new(config.turn.realm.clone(), externals, observer);
    server::run(config.clone(), monitor.clone(), &service).await?;

    let ctr = Controller::new(config.clone(), monitor, service);
    api::start_controller_service(&config, &ctr).await?;
    Ok(())
}
