use tokio::time::Instant;
use super::Addr;

pub struct Channel {
    pub timer: Instant,
    pub bond: [Option<Addr>; 2],
}

impl Channel {
    pub fn new(a: &Addr) -> Self {
        Self {
            bond: [Some(a.clone()), None],
            timer: Instant::now(),
        }
    }
    
    pub fn includes(&self, a: &Addr) -> bool {
        self.bond.contains(&Some(a.clone()))
    }

    pub fn is_half(&self) -> bool {
        self.bond[1].is_none()
    }

    pub fn up(&mut self, a: &Addr) {
        self.bond[1] = Some(a.clone())
    }

    pub fn refresh(&mut self) {
        self.timer = Instant::now();
    }
}
