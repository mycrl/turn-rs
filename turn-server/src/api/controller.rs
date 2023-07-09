use tokio::time::Instant;
use serde::*;
use axum::{
    extract::Query,
    extract::State,
    Json,
};

use std::{
    net::SocketAddr,
    sync::Arc,
};

use crate::{
    config::*,
    server::{
        Store,
        Monitor,
    },
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
    /// ```ignore
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
    config: Arc<Config>,
    router: Arc<Router>,
    monitor: Monitor,
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
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// Controller::new(service.get_router(), config, monitor);
    /// ```
    pub fn new(
        config: Arc<Config>,
        monitor: Monitor,
        router: Arc<Router>,
    ) -> Arc<Self> {
        Arc::new(Self {
            timer: Instant::now(),
            monitor,
            router,
            config,
        })
    }

    /// get server status.
    ///
    /// # Example
    ///
    /// ```ignore
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
            port_allocated: this.router.len() as u16,
            port_capacity: this.router.capacity() as u16,
            interfaces: this.config.turn.interfaces.clone(),
        })
    }

    /// Get a list of sockets
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let workers_js = ctr.get_report().await;
    /// ```
    pub async fn get_report(
        State(this): State<&Self>,
        Query(pars): Query<Qiter>,
    ) -> Json<Vec<(SocketAddr, Store)>> {
        let skip = pars.skip.unwrap_or(0);
        let limit = pars.limit.unwrap_or(20);
        Json(this.monitor.get_nodes(skip, limit))
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
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
    ) -> Json<Vec<(String, Vec<SocketAddr>)>> {
        let skip = pars.skip.unwrap_or(0);
        let limit = pars.limit.unwrap_or(20);
        Json(this.router.get_users(skip, limit))
    }

    /// Get node information
    ///
    /// This interface can obtain the user's basic information and assigned
    /// information, including the survival time.
    ///
    /// # Example
    ///
    /// ```ignore
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
        Json(this.router.get_node(&Arc::new(pars.addr)).map(INode::from))
    }

    /// Delete a node under the user.
    ///
    /// This will cause all information of the current node to be deleted,
    /// including the binding relationship, and at the same time terminate the
    /// INodeion of the current node and stop forwarding data.
    ///
    /// # Example
    ///
    /// ```ignore
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
                .is_some()
        )
    }
}
