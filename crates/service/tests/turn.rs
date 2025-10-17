use std::net::{Ipv4Addr, SocketAddr};

use anyhow::Result;
use codec::{
    crypto::{Password, generate_password},
    message::attributes::PasswordAlgorithm,
};
use turn_server_service::{routing::Router, session::ports::PortRange, Service, ServiceHandler, ServiceOptions};

#[derive(Default, Clone)]
struct Handler;

impl ServiceHandler for Handler {
    async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
        if username != "USERNAME" {
            return None;
        }

        Some(generate_password(
            "USERNAME", "PASSWORD", "REALM", algorithm,
        ))
    }
}

#[test]
fn test_turn_server_service() -> Result<()> {
    let service = Service::new(ServiceOptions {
        port_range: PortRange::default(),
        software: "SOFTWARE".to_string(),
        realm: "REALM".to_string(),
        interfaces: vec![SocketAddr::from((Ipv4Addr::LOCALHOST, 3478))],
        handler: Handler::default(),
    });

    let router = service.make_router(
        SocketAddr::from((Ipv4Addr::LOCALHOST, 3478)),
        SocketAddr::from((Ipv4Addr::LOCALHOST, 3478)),
    );

    Ok(())
}

async fn test_peer(router: Router<Handler>) -> Result<()> {
    {
        router.route(bytes, address).await?;
    }

    Ok(())
}
