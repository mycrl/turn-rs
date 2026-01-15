use std::fs::{create_dir_all, metadata};

use anyhow::Result;
use fern::{DateBased, Dispatch};
use turn_server::config::Config;

pub fn init(config: &Config) -> Result<()> {
    let mut logger =
        Dispatch::new()
            .level(config.log.level.into())
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "[{}] - ({}) - {}",
                    record.level(),
                    record.file_static().unwrap_or("*"),
                    message
                ))
            });

    if config.log.stdout {
        logger = logger.chain(std::io::stdout());
    }

    if let Some(path) = &config.log.file_directory {
        if metadata(path).is_err() {
            create_dir_all(path)?;
        }

        logger = logger.chain(DateBased::new(path, "turn-server-%Y-%m-%d.log"))
    }

    logger.apply()?;

    Ok(())
}
