use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use ahash::AHashMap;
use tokio::time::sleep;

use crate::config::Config;

pub struct Node {
    online: bool,
    timer: Instant,
}

pub struct Cluster(Mutex<AHashMap<SocketAddr, Node>>);

impl Cluster {
    pub fn new(cfg: Arc<Config>) -> Arc<Self> {
        let mut nodes = AHashMap::with_capacity(cfg.cluster.nodes.len());
        for item in &cfg.cluster.nodes {
            nodes.insert(
                *item,
                Node {
                    timer: Instant::now(),
                    online: false,
                },
            );
        }

        let this = Arc::new(Self(Mutex::new(nodes)));
        let this_ = this.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(10)).await;
                for (addr, item) in this_.0.lock().unwrap().iter_mut() {
                    if item.timer.elapsed().as_secs() >= 15 {
                        log::info!("node offline: addr={}", addr);
                        item.online = false;
                    }
                }
            }
        });

        this
    }

    pub fn get_onlines(&self) -> Vec<SocketAddr> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, v)| v.online)
            .map(|(k, _)| *k)
            .collect()
    }

    pub fn update(&self, addr: &SocketAddr) {
        if let Some(node) = self.0.lock().unwrap().get_mut(addr) {
            node.timer = Instant::now();

            if !node.online {
                log::info!("node online: addr={}", addr);
                node.online = true;
            }
        }
    }
}
