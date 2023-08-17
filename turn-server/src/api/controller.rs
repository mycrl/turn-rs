use std::{net::SocketAddr, sync::Arc};

use super::payload::{Node, Stats, SOFTWARE};
use crate::{config::*, server::*};

use axum::{extract::Query, extract::State, Json};
use serde::Deserialize;
use tokio::time::Instant;
use turn_rs::Service;

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
    service: Service,
    monitor: Monitor,
    timer: Instant,
}

impl Controller {
    pub fn new(config: Arc<Config>, monitor: Monitor, service: Service) -> Arc<Self> {
        Arc::new(Self {
            timer: Instant::now(),
            monitor,
            service,
            config,
        })
    }

    /// get server status
    ///
    /// ```base
    /// curl -X GET {hostname}/stats #application/json
    /// ```
    pub async fn get_stats(State(this): State<&Self>) -> Json<Stats> {
        let router = this.service.get_router();
        Json(Stats {
            software: SOFTWARE.to_string(),
            uptime: this.timer.elapsed().as_secs(),
            realm: this.config.turn.realm.clone(),
            port_allocated: router.len() as u16,
            port_capacity: router.capacity() as u16,
            interfaces: this.config.turn.interfaces.clone(),
        })
    }

    /// get a list of sockets
    ///
    /// ```
    /// curl -X GET {hostname}/report?skip=0&limit=20 #application/json
    /// ```
    pub async fn get_report(
        State(this): State<&Self>,
        Query(pars): Query<Qiter>,
    ) -> Json<Vec<(SocketAddr, Store)>> {
        let skip = pars.skip.unwrap_or(0);
        let limit = pars.limit.unwrap_or(20);
        Json(this.monitor.get_nodes(skip, limit))
    }

    /// get user list
    ///
    /// ```
    /// curl -X GET {hostname}/users?skip=0&limit=20 #application/json
    /// ```
    pub async fn get_users(
        State(this): State<&Self>,
        Query(pars): Query<Qiter>,
    ) -> Json<Vec<(String, Vec<SocketAddr>)>> {
        let router = this.service.get_router();
        let skip = pars.skip.unwrap_or(0);
        let limit = pars.limit.unwrap_or(20);
        Json(router.get_users(skip, limit))
    }

    /// get node information
    ///
    /// ```
    /// curl -X GET {hostname}/node?addr=127.0.0.1 #application/json
    /// ```
    pub async fn get_node(
        State(this): State<&Self>,
        Query(pars): Query<AddrParams>,
    ) -> Json<Option<Node>> {
        let router = this.service.get_router();
        Json(router.get_node(&Arc::new(pars.addr)).map(Node::from))
    }

    /// delete a node under the user
    ///
    /// ```
    /// curl -X DELETE {hostname}/node?addr=127.0.0.1
    /// ```
    pub async fn remove_node(
        State(this): State<&Self>,
        Query(pars): Query<AddrParams>,
    ) -> Json<bool> {
        let router = this.service.get_router();
        Json(router.remove(&Arc::new(pars.addr)).is_some())
    }
}
