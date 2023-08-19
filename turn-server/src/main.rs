#[global_allocator]
#[cfg(target_os = "windows")]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[global_allocator]
#[cfg(not(target_os = "windows"))]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use std::sync::Arc;
use turn_server::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Arc::new(Config::load()?);
    simple_logger::init_with_level(config.log.level.as_level())?;
    turn_server::server_main(config).await
}
