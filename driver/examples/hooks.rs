use std::net::SocketAddr;

use async_trait::async_trait;
use turn_driver::hooks::start_hooks_service;
use turn_rs::Observer;

struct SimpleObserver;

#[async_trait]
impl Observer for SimpleObserver {
    async fn get_password(&self, addr: &SocketAddr, name: &str) -> Option<String> {
        println!("get password: addr={}, name={}", addr, name);
        Some("test".to_string())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    start_hooks_service("0.0.0.0:3000".parse()?, SimpleObserver).await?;
    Ok(())
}
