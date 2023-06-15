use tokio::time::Instant;
use serde::*;
use axum::{
    extract::Query,
    extract::State,
    Json,
};

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};

use crate::{
    monitor::MonitorWorker,
    monitor::Monitor,
    config::*,
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
    /// The listening interfaces of the turn server.
    interfaces: Vec<Interface>,
    /// The running time of the server, in seconds.
    uptime: u64,
    /// Turn server port pool capacity.
    port_capacity: u16,
    /// The number of ports that the turn server has classified.
    port_allocated: u16,
    /// The partition where the turn server resides.
    realm: String,
}

/// Represents a turn INodeion processing thread
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

impl From<&MonitorWorker> for Worker {
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
    fn from(value: &MonitorWorker) -> Self {
        Self {
            receive_packets: value.receive_packets,
            failed_packets: value.failed_packets,
            send_packets: value.send_packets,
        }
    }
}

/// node information in the turn server
#[derive(Serialize)]
pub struct INode {
    /// Username for the current INodeion.
    username: String,
    /// The user key for the current INodeion.
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

impl From<Node> for INode {
    /// # Example
    ///
    /// ```no_run
    /// let node = Node {
    ///     ...
    /// };
    ///
    /// let INode = INode::from(node.clone());
    /// assert_eq!(INode.username, node.username);
    /// assert_eq!(INode.password, node.password);
    /// assert_eq!(INoder.lifetime, node.lifetime);
    /// ```
    fn from(value: Node) -> Self {
        INode {
            timer: value.timer.elapsed().as_secs(),
            username: value.username.clone(),
            allocated_channels: value.channels,
            allocated_ports: value.ports,
            password: value.password,
            lifetime: value.lifetime,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AddrParams {
    addr: SocketAddr,
}

#[derive(Debug, Deserialize)]
pub struct Qiter {
    skip: Option<usize>,
    limit: Option<usize>,
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
        monitor: Monitor,
        router: Arc<Router>,
        config: Arc<Config>,
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
    pub async fn get_stats(State(this): State<&Self>) -> Json<Stats> {
        Json(Stats {
            software: SOFTWARE.to_string(),
            uptime: this.timer.elapsed().as_secs(),
            realm: this.config.turn.realm.clone(),
            port_allocated: this.router.len().await as u16,
            port_capacity: this.router.capacity().await as u16,
            interfaces: this.config.turn.interfaces.clone(),
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
    pub async fn get_workers(
        State(this): State<&Self>,
    ) -> Json<HashMap<u8, Worker>> {
        let workers = this
            .monitor
            .get_workers()
            .await
            .iter()
            .map(|(k, v)| (*k, Worker::from(v)))
            .collect();
        Json(workers)
    }

    /// Get user list.
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
    pub async fn get_users(
        State(this): State<&Self>,
        Query(pars): Query<Qiter>,
    ) -> Json<HashMap<String, Vec<SocketAddr>>> {
        let skip = pars.skip.unwrap_or(0);
        let limit = pars.limit.unwrap_or(20);
        Json(
            this.router
                .get_users(skip, limit)
                .await
                .into_iter()
                .collect(),
        )
    }

    /// Get node information
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
    /// // let user_js = ctr.get_node(addr).await;
    /// ```
    pub async fn get_node(
        State(this): State<&Self>,
        Query(pars): Query<AddrParams>,
    ) -> Json<Option<INode>> {
        Json(
            this.router
                .get_node(&Arc::new(pars.addr))
                .await
                .map(INode::from),
        )
    }

    /// Delete a node under the user.
    ///
    /// This will cause all information of the current node to be deleted,
    /// including the binding relationship, and at the same time terminate the
    /// INodeion of the current node and stop forwarding data.
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
    /// // let remove_node_js = ctr.remove_user(addr).await;
    /// ```
    #[rustfmt::skip]
    pub async fn remove_node(
        State(this): State<&Self>,
        Query(pars): Query<AddrParams>,
    ) -> Json<bool> {
        Json(
            this.router
                .remove(&Arc::new(pars.addr))
                .await
                .is_some()
        )
    }
}
