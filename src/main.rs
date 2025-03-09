#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::sync::Arc;

use turn_server::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Arc::new(Config::load()?);
    simple_logger::init_with_level(config.log.level.as_level())?;

    if config.turn.interfaces.is_empty() {
        log::warn!(
            "No interfaces are bound, no features are enabled, it's just a program without any functionality :-)"
        );

        return Ok(());
    }

    turn_server::startup(config).await
}
