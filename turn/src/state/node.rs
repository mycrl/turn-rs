use tokio::time::Instant;
use std::sync::Arc;
// use super::util;

#[derive(Debug)]
pub struct Node {
    pub ports: Vec<u16>,
    pub channels: Vec<u16>,
    pub group: u32,
    timer: Instant,
    lifetime: u64,
    key: Arc<[u8; 16]>
}

impl Node {
    pub fn new(group: u32, key: [u8; 16]) -> Self {
        Self {
            channels: Vec::with_capacity(5),
            ports: Vec::with_capacity(10),
            timer: Instant::now(),
            key: Arc::new(key),
            lifetime: 600,
            group,
        }
    }

    // pub fn remove_channel(&mut self, c: u16) -> Option<()> {
    //     let index = util::first_index(&self.channels, c)?;
    //     self.channels.swap_remove(index);
    //     Some(())
    // }
    
    // pub fn remove_port(&mut self, p: u16) -> Option<()> {
    //     let index = util::first_index(&self.ports, p)?;
    //     self.ports.swap_remove(index);
    //     Some(())
    // }

    pub fn is_timeout(&self) -> bool {
        self.timer.elapsed().as_secs() >= self.lifetime
    }

    pub fn set_lifetime(&mut self, delay: u32) {
        self.lifetime = delay as u64;
        self.timer = Instant::now();
    }

    pub fn get_key(&self) -> Arc<[u8; 16]> {
        self.key.clone()
    }
}
