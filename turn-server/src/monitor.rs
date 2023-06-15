use std::{
    collections::HashMap,
    sync::atomic::*,
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
    workers: Arc<Mutex<HashMap<u8, MonitorWorker>>>,
    sender: Sender<(u8, Payload)>,
    index: AtomicU8,
}

impl Monitor {
    /// Create a monitoring instance
    ///
    /// Creating a monitoring instance requires a number of workers, so that the
    /// monitoring instance can create a worker list and default information
    /// based on the number of workers.
    pub fn new() -> Self {
        let (sender, mut receiver) = channel(2);
        let workers: Arc<Mutex<HashMap<u8, MonitorWorker>>> =
            Default::default();
        let workers_ = workers.clone();
        tokio::spawn(async move {
            while let Some((i, payload)) = receiver.recv().await {
                if let Some(w) = workers_.lock().await.get_mut(&i) {
                    w.change(payload);
                }
            }
        });

        Self {
            index: AtomicU8::new(0),
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
    pub async fn get_sender(&self) -> Arc<MonitorSender> {
        let index = self.index.load(Ordering::Relaxed);
        self.workers
            .lock()
            .await
            .insert(index, MonitorWorker::default());

        Arc::new(MonitorSender {
            sender: self.sender.clone(),
            index,
        })
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
    pub async fn get_workers(&self) -> MutexGuard<HashMap<u8, MonitorWorker>> {
        self.workers.lock().await
    }
}

trait OverflowingAdd {
    fn add(&mut self, num: u64);
}

impl OverflowingAdd for u64 {
    fn add(&mut self, num: u64) {
        *self = self.overflowing_add(num).0;
    }
}

/// Worker independent monitoring statistics
#[derive(Default)]
pub struct MonitorWorker {
    pub receive_packets: u64,
    pub send_packets: u64,
    pub failed_packets: u64,
}

impl MonitorWorker {
    /// update status information
    fn change(&mut self, payload: Payload) {
        match payload {
            Payload::Receive => self.receive_packets.add(1),
            Payload::Failed => self.failed_packets.add(1),
            Payload::Send => self.send_packets.add(1),
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
    /// TODO: Delivery of updates is not guaranteed.
    pub fn send(&self, payload: Payload) {
        let _ = self.sender.try_send((self.index, payload));
    }
}
