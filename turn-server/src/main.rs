use std::sync::Arc;
use mimalloc::MiMalloc;
use turn_server::config::Config;

// use mimalloc for global.
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Arc::new(Config::load()?);
    simple_logger::init_with_level(config.log.level.as_level())?;
    turn_server::server_main(config).await
}
