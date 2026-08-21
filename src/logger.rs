use std::{
    fmt::Arguments,
    fs::{create_dir_all, metadata},
    net::SocketAddr,
    time::Duration,
};

use anyhow::Result;
use fern::{DateBased, Dispatch, FormatCallback};
use log::Record;
use serde::Serialize;
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    sync::mpsc::{Sender, channel},
    time::timeout,
};

use turn_server::config::Config;

/// Timeout for connecting to the Vector endpoint.
const VECTOR_CONNECT_TIMEOUT: Duration = Duration::from_millis(100);

/// Capacity for the Vector channel.
const VECTOR_CHANNEL_CAPACITY: usize = 1024;

#[derive(Serialize)]
struct VectorEvent<'a> {
    message: String,
    level: &'a str,
    target: &'a str,
    file: &'a str,
}

struct VectorSink {
    sender: Sender<Vec<u8>>,
}

impl VectorSink {
    async fn new(endpoint: SocketAddr) -> Result<Self> {
        let mut stream = timeout(VECTOR_CONNECT_TIMEOUT, TcpStream::connect(endpoint)).await??;
        stream.set_nodelay(true)?;

        let (sender, mut receiver) = channel::<Vec<u8>>(VECTOR_CHANNEL_CAPACITY);

        tokio::spawn(async move {
            while let Some(payload) = receiver.recv().await {
                if stream.write_all(&payload).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self { sender })
    }

    fn emit(&self, record: &log::Record) {
        if let Ok(mut payload) = serde_json::to_vec(&VectorEvent {
            message: record.args().to_string(),
            level: record.level().as_str(),
            target: record.target(),
            file: record.file_static().unwrap_or("*"),
        }) {
            payload.push(b'\n');

            let _ = self.sender.try_send(payload);
        };
    }
}

fn text_format(out: FormatCallback, message: &Arguments, record: &Record) {
    out.finish(format_args!(
        "[{}] - ({}) - {}",
        record.level(),
        record.file_static().unwrap_or("*"),
        message
    ))
}

pub async fn init_with_config(config: &Config) -> Result<()> {
    let mut logger = Dispatch::new().level(config.log.level.into());

    if config.log.stdout {
        logger = logger.chain(Dispatch::new().format(text_format).chain(std::io::stdout()));
    }

    if let Some(path) = &config.log.file_directory {
        if metadata(path).is_err() {
            create_dir_all(path)?;
        }

        logger = logger.chain(
            Dispatch::new()
                .format(text_format)
                .chain(DateBased::new(path, "turn-server-%Y-%m-%d.log")),
        );
    }

    if let Some(vector) = &config.log.vector {
        // Vector has no application SDK; the integration surface is a source.
        // This sink speaks newline-delimited JSON to the `socket` source.
        let sink = VectorSink::new(vector.endpoint).await?;

        logger = logger.chain(fern::Output::call(move |record| sink.emit(record)));
    }

    logger.apply()?;

    Ok(())
}
