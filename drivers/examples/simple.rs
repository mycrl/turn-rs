use std::net::SocketAddr;

use async_trait::async_trait;
use clap::Parser;
use tabled::{Table, Tabled};
use turn_driver::{start_hooks_server, Controller, Events, Hooks, SessionAddr, Transport};

struct HooksImpl;

#[async_trait]
impl Hooks for HooksImpl {
    async fn auth(
        &self,
        addr: &SessionAddr,
        username: &str,
        realm: &str,
        nonce: &str,
    ) -> Option<&str> {
        println!(
            "auth: address={:?}, interface={:?}, username={:?}, realm={}, nonce={}",
            addr.address, addr.interface, username, realm, nonce
        );

        Some("test")
    }

    async fn on(&self, event: &Events, realm: &str, nonce: &str) {
        println!("event={:?}, realm={}, nonce={}", event, realm, nonce)
    }
}

#[derive(Tabled)]
struct BaseInfo {
    software: String,
    uptime: u64,
    port_allocated: u16,
    port_capacity: u16,
}

#[derive(Tabled)]
struct Interface {
    transport: String,
    bind: SocketAddr,
    external: SocketAddr,
}

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    bind: SocketAddr,
    #[arg(long)]
    server: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let controller = Controller::new(&cli.server).unwrap();
    tokio::spawn(start_hooks_server(cli.bind, HooksImpl));

    if let Some(info) = controller.get_info().await {
        println!("Base info:");
        println!(
            "{}\r\n",
            Table::new([BaseInfo {
                software: info.payload.software,
                uptime: info.payload.uptime,
                port_allocated: info.payload.port_allocated,
                port_capacity: info.payload.port_capacity,
            }])
            .to_string()
        );

        println!("Interfaces:");
        println!(
            "{}",
            Table::new(
                info.payload
                    .interfaces
                    .into_iter()
                    .map(|it| Interface {
                        transport: if it.transport == Transport::UDP {
                            "UDP"
                        } else {
                            "TCP"
                        }
                        .to_string(),
                        external: it.external,
                        bind: it.bind,
                    })
                    .collect::<Vec<Interface>>()
            )
            .to_string()
        );
    } else {
        println!("turn server not runing!");
        return;
    }
}
