use tokio::net::UdpSocket;
use super::config::Config;
use turn_rs::{
    Service,
    Processor,
};

use std::{
    collections::HashMap,
    sync::Arc,
};

use tokio::sync::{
    Mutex,
    MutexGuard,
    mpsc::*,
};

/// The type of information passed in the monitoring channel
#[derive(Debug, Clone)]
pub enum Payload {
    Receive,
    Send,
    Failed,
}

/// worker cluster monitor
pub struct Monitor {
    workers: Arc<Mutex<HashMap<u8, WorkMonitor>>>,
    sender: Sender<(u8, Payload)>,
}

impl Monitor {
    /// Create a monitoring instance
    ///
    /// Creating a monitoring instance requires a number of workers, so that the
    /// monitoring instance can create a worker list and default information
    /// based on the number of workers.
    pub fn new(size: usize) -> Self {
        let (sender, mut receiver) = channel(2);
        let mut workers = HashMap::with_capacity(size);
        for i in 0..size {
            workers.insert(i as u8, WorkMonitor::default());
        }

        let workers = Arc::new(Mutex::new(workers));
        let workers_c = workers.clone();
        tokio::spawn(async move {
            while let Some((i, payload)) = receiver.recv().await {
                if let Some(w) = workers_c.lock().await.get_mut(&i) {
                    w.change(payload);
                }
            }
        });

        Self {
            workers,
            sender,
        }
    }

    /// get signal sender
    ///
    /// The signal sender can notify the monitoring instance to update internal
    /// statistics.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let monitor = Monitor::new(2);
    /// let sender = monitor.get_sender(0);
    ///
    /// sender.send(Payload::Receive);
    /// ```
    pub fn get_sender(&self, index: usize) -> MonitorSender {
        MonitorSender {
            sender: self.sender.clone(),
            index: index as u8,
        }
    }

    /// get all workers
    ///
    /// Get a list of workers with worker stats.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let monitor = Monitor::new(2);
    /// let workers = monitor.get_workers().await;
    ///
    /// assert_eq!(workers.get(0).unwrap().receive_packets, 0);
    /// ```
    pub async fn get_workers(&self) -> MutexGuard<HashMap<u8, WorkMonitor>> {
        self.workers.lock().await
    }
}

/// Worker independent monitoring statistics
#[derive(Default)]
pub struct WorkMonitor {
    pub receive_packets: u64,
    pub send_packets: u64,
    pub failed_packets: u64,
}

impl WorkMonitor {
    /// update status information
    #[rustfmt::skip]
    fn change(&mut self, payload: Payload) {
        match payload {
            Payload::Receive => self.receive_packets = self.receive_packets.overflowing_add(1).0,
            Payload::Failed => self.failed_packets = self.failed_packets.overflowing_add(1).0,
            Payload::Send => self.send_packets = self.send_packets.overflowing_add(1).0,
        }
    }
}

/// monitor sender
///
/// It is held by each worker, and status information can be sent to the
/// monitoring instance through this instance to update the internal statistical
/// information of the monitor.
pub struct MonitorSender {
    sender: Sender<(u8, Payload)>,
    index: u8,
}

impl MonitorSender {
    pub fn send(&self, payload: Payload) {
        let _ = self.sender.try_send((self.index, payload));
    }
}

/// thread poll.
///
/// read the data packet from the UDP socket and hand
/// it to the proto for processing, and send the processed
/// data packet to the specified address.
///
/// # Example
///
/// ```no_run
/// let c = env::Environment::generate()?;
/// let t = broker::Broker::new(&c).await?;
/// let s = state::State::new(t);
///
/// let thread_local = SocketLocal {
///     state: s,
///     conf: c,
/// };
///
/// let s = Arc::new(UdpSocket::bind(c.listen).await?);
/// tokio::spawn(start(thread_local, &s));
/// ```
pub async fn start(
    sender: MonitorSender,
    mut processor: Processor,
    socket: Arc<UdpSocket>,
) -> anyhow::Result<()> {
    let mut reader = vec![0u8; 4096];

    loop {
        if let Ok((size, addr)) = socket.recv_from(&mut reader).await {
            if size >= 4 {
                sender.send(Payload::Receive);
                if let Ok(Some((buf, target))) =
                    processor.process(&reader[..size], addr).await
                {
                    socket.send_to(buf, target.as_ref()).await?;
                    sender.send(Payload::Send);
                } else {
                    sender.send(Payload::Failed);
                }
            }
        }
    }
}

/// get thread num.
///
/// by default, the thread pool is used to process UDP packets.
/// because UDP uses SysCall to ensure concurrency security,
/// using multiple threads may not bring a very significant
/// performance improvement, but setting the number of CPU
/// cores can process data to the greatest extent package.
fn get_threads(threads: Option<usize>) -> usize {
    threads.unwrap_or_else(num_cpus::get)
}

/// start udp server.
///
/// create a specified number of threads,
/// each thread processes udp data separately.
///
/// # Example
///
/// ```no_run
/// let c = env::Environment::generate()?;
/// let t = broker::Broker::new(&c).await?;
/// let s = state::State::new(t);
///
/// // run(c, s).await?
/// ```
pub async fn run(
    service: &Service,
    config: Arc<Config>,
) -> anyhow::Result<Monitor> {
    let socket = Arc::new(UdpSocket::bind(config.bind).await?);
    let threads = get_threads(config.threads);
    let monitor = Monitor::new(threads);

    for index in 0..threads {
        tokio::spawn(start(
            monitor.get_sender(index),
            service.get_processor(),
            socket.clone(),
        ));
    }

    log::info!("threads size {} is runing", threads);
    log::info!("udp bind to {}", config.bind);

    Ok(monitor)
}
