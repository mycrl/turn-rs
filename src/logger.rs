use std::{
    fmt::Arguments,
    fs::{create_dir_all, metadata},
    io::Write,
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

use anyhow::Result;
use fern::{DateBased, Dispatch, FormatCallback};
use log::Record;
use parking_lot::Mutex;
use serde::Serialize;
use turn_server::config::Config;

const VECTOR_CONNECT_TIMEOUT: Duration = Duration::from_millis(100);
const VECTOR_WRITE_TIMEOUT: Duration = Duration::from_millis(100);
const VECTOR_RETRY_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Serialize)]
struct VectorEvent<'a> {
    message: String,
    level: &'a str,
    target: &'a str,
    file: &'a str,
}

struct VectorSink {
    endpoint: SocketAddr,
    inner: Mutex<VectorSinkInner>,
}

struct VectorSinkInner {
    stream: Option<TcpStream>,
    last_error: Option<Instant>,
}

impl VectorSink {
    fn new(endpoint: SocketAddr) -> Self {
        Self {
            endpoint,
            inner: Mutex::new(VectorSinkInner {
                stream: None,
                last_error: None,
            }),
        }
    }

    fn emit(&self, record: &log::Record) {
        let Ok(mut payload) = serde_json::to_vec(&VectorEvent {
            message: record.args().to_string(),
            level: record.level().as_str(),
            target: record.target(),
            file: record.file_static().unwrap_or("*"),
        }) else {
            return;
        };

        payload.push(b'\n');

        let mut inner = self.inner.lock();

        if inner.stream.is_none() {
            if inner
                .last_error
                .is_some_and(|at| at.elapsed() < VECTOR_RETRY_INTERVAL)
            {
                return;
            }

            match TcpStream::connect_timeout(&self.endpoint, VECTOR_CONNECT_TIMEOUT) {
                Ok(stream) => {
                    let _ = stream.set_nodelay(true);
                    let _ = stream.set_write_timeout(Some(VECTOR_WRITE_TIMEOUT));
                    inner.stream = Some(stream);
                    inner.last_error = None;
                }
                Err(_) => {
                    inner.last_error = Some(Instant::now());
                    return;
                }
            }
        }

        if let Some(stream) = inner.stream.as_mut()
            && stream.write_all(&payload).is_err()
        {
            inner.stream = None;
            inner.last_error = Some(Instant::now());
        }
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

pub fn init(config: &Config) -> Result<()> {
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
        let sink = VectorSink::new(vector.endpoint);

        logger = logger.chain(fern::Output::call(move |record| sink.emit(record)));
    }

    logger.apply()?;

    Ok(())
}
