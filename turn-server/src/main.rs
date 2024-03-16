#[global_allocator]
#[cfg(not(feature = "system_allocator"))]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::sync::Arc;
use turn_server::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Arc::new(Config::load()?);
    simple_logger::init_with_level(config.log.level.as_level())?;
    turn_server::server_main(config).await
}
