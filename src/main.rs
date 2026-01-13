mod logger;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use turn_server::config::Config;

fn main() -> anyhow::Result<()> {
    let config = Config::load()?;

    logger::init(&config)?;

    if config.server.interfaces.is_empty() {
        log::warn!(
            "No interfaces are bound, no features are enabled, it's just a program without any functionality :-)"
        );

        return Ok(());
    }

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.server.max_threads)
        .enable_all()
        .build()?
        .block_on(turn_server::start_server(config))
}
