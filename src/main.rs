mod args;
mod logger;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use args::Args;
use turn_server::config::Config;

fn main() -> anyhow::Result<()> {
    let args = Args::parse()?;
    let config = Config::load(&args.config)?;

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.server.max_threads)
        .enable_all()
        .build()?
        .block_on(async {
            logger::init_with_config(&config).await?;

            if config.server.interfaces.is_empty() {
                log::warn!(
                    "No interfaces are bound, no features are enabled, it's just a program without any functionality :-)"
                );

                return Ok(());
            }

            turn_server::start_server(config).await
        })
}
