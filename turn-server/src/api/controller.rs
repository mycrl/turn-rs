use tokio::time::Instant;
use serde::Serialize;
use anyhow::Result;
use axum::Json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    fs,
};

use crate::{
    server::WorkMonitor,
    server::Monitor,
    config::Config,
};

use turn_rs::{
    Router,
    Node,
};

#[rustfmt::skip]
static SOFTWARE: &str = concat!(
    env!("CARGO_PKG_NAME"), 
    ":",
    env!("CARGO_PKG_VERSION")
);

#[derive(Serialize)]
pub struct Stats {
    /// Software information, usually a name and version string.
    software: String,
    /// The listening address of the turn server.
    bind_address: SocketAddr,
    /// The external address of the turn server.
    external_address: SocketAddr,
    /// The running time of the server, in seconds.
    uptime: u64,
    /// Turn server port pool capacity.
    port_capacity: u16,
    /// The number of ports that the turn server has classified.
    port_allocated: u16,
    /// The partition where the turn server resides.
    realm: String,
}

/// Represents a turn session processing thread
#[derive(Serialize)]
pub struct Worker {
    /// The total number of data packets that the turn server has received so
    /// far.
    receive_packets: u64,
    /// The total number of packets sent by the turn server so far.
    send_packets: u64,
    /// The total number of packets that the turn server failed to process.
    failed_packets: u64,
}

impl From<&WorkMonitor> for Worker {
    /// start udp server.
    ///
    /// create a specified number of threads,
    /// each thread processes udp data separately.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let mworker = WorkerMonitor {
    ///     receive_packets: 10,
    ///     failed_packets: 0,
    ///     send_packets: 10,
    /// };
    ///
    /// let worker = Worker::from(&mworker);
    /// assert_eq!(worker.receive_packets, 10);
    /// assert_eq!(worker.failed_packets, 0);
    /// assert_eq!(worker.send_packets, 10);
    /// ```
    fn from(value: &WorkMonitor) -> Self {
        Self {
            receive_packets: value.receive_packets,
            failed_packets: value.failed_packets,
            send_packets: value.send_packets,
        }
    }
}

/// Session information in the turn server
#[derive(Serialize)]
pub struct Sess {
    /// Username for the current session.
    username: String,
    /// The user key for the current session.
    password: String,
    /// The lifetime of the current user.
    lifetime: u64,
    /// The active time of the current user, in seconds.
    timer: u64,
    /// List of assigned channel numbers.
    allocated_channels: Vec<u16>,
    /// List of assigned port numbers.
    allocated_ports: Vec<u16>,
}

impl From<Node> for Sess {
    /// start udp server.
    ///
    /// create a specified number of threads,
    /// each thread processes udp data separately.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let node = Node {
    ///     ...
    /// };
    ///
    /// let sess = Sess::from(node.clone());
    /// assert_eq!(sess.username, node.username);
    /// assert_eq!(sess.password, node.password);
    /// assert_eq!(sessr.lifetime, node.lifetime);
    /// ```
    fn from(value: Node) -> Self {
        Sess {
            timer: value.timer.elapsed().as_secs(),
            username: value.username.clone(),
            allocated_channels: value.channels,
            allocated_ports: value.ports,
            password: value.password,
            lifetime: value.lifetime,
        }
    }
}

/// controller
///
/// It is possible to control the turn server and obtain server internal
/// information and reports through the controller.
pub struct Controller {
    monitor: Monitor,
    config: Arc<Config>,
    router: Arc<Router>,
    timer: Instant,
}

impl Controller {
    /// Create a controller.
    ///
    /// Controllers require external routing and thread monitoring instances, as
    /// well as configuration information.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// Controller::new(service.get_router(), config, monitor);
    /// ```
    pub fn new(
        router: Arc<Router>,
        config: Arc<Config>,
        monitor: Monitor,
    ) -> Arc<Self> {
        Arc::new(Self {
            timer: Instant::now(),
            monitor,
            config,
            router,
        })
    }

    /// get server status.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let state_js = ctr.get_stats().await;
    /// ```
    pub async fn get_stats(&self) -> Json<Stats> {
        Json(Stats {
            software: SOFTWARE.to_string(),
            bind_address: self.config.bind,
            external_address: self.config.external,
            uptime: self.timer.elapsed().as_secs(),
            realm: self.config.realm.clone(),
            port_allocated: self.router.len().await as u16,
            port_capacity: self.router.capacity().await as u16,
        })
    }

    /// Get a list of workers
    ///
    /// Workers are bound to the internal threads of the server. Through this
    /// interface, you can get how many threads currently exist in the server,
    /// and how much data processing capacity each thread has.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let workers_js = ctr.get_workers().await;
    /// ```
    pub async fn get_workers(&self) -> Json<HashMap<u8, Worker>> {
        let workers = self
            .monitor
            .get_workers()
            .await
            .iter()
            .map(|(k, v)| (*k, Worker::from(v)))
            .collect();
        Json(workers)
    }

    /// get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub async fn get_users(&self) -> Json<Vec<(String, Vec<SocketAddr>)>> {
        Json(self.router.get_users().await)
    }

    /// Get user information
    ///
    /// This interface can obtain the user's basic information and assigned
    /// information, including the survival time.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// let addr = "127.0.0.1:8080".parse().unwrap();
    /// // let user_js = ctr.get_user(addr).await;
    /// ```
    pub async fn get_user(&self, addr: SocketAddr) -> Json<Option<Sess>> {
        Json(self.router.get_node(&Arc::new(addr)).await.map(Sess::from))
    }
}

/// external controller
///
/// The external controller is used for the turn server to send requests to the
/// outside and notify or obtain information necessary for operation.
pub struct ExtController {
    static_certs: HashMap<String, String>,
    config: Arc<Config>,
}

impl ExtController {
    /// Create an external controller
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = Config::new()
    /// // let ext_ctr = ExtController::new(config);
    /// ```
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            static_certs: config
                .cert_file
                .as_ref()
                .map(|f| fs::read_to_string(&f).unwrap_or("".to_string()))
                .map(|s| toml::from_str(&s).unwrap())
                .unwrap_or_else(|| HashMap::new()),
            config,
        }
    }

    /// request external authentication.
    ///
    /// This interface will first try to find the internal static certificate
    /// table, if not found, then request the external interface for
    /// authentication.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = Config::new()
    /// let ext_ctr = ExtController::new(config);
    ///
    /// let addr = "127.0.0.1:8080".parse().unwrap();
    /// // let key = ext_ctr.auth(&addr, "test").await?;
    /// ```
    pub async fn auth(&self, addr: &SocketAddr, name: &str) -> Result<String> {
        if let Some(v) = self.static_certs.get(name) {
            return Ok(v.clone());
        }

        Ok(reqwest::get(format!(
            "{}/auth?addr={}&name={}",
            self.config.ext_controller_bind, addr, name
        ))
        .await?
        .text()
        .await?)
    }
}
