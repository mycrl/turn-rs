use tokio::time::Instant;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use super::Addr;
use rand::{
    distributions::Alphanumeric, 
    thread_rng, 
    Rng
};

pub struct Nonce {
    raw: Arc<String>,
    timer: Instant
}

pub struct NonceTable {
    raw: RwLock<HashMap<Addr, Nonce>>
}

impl NonceTable {
    pub fn new() -> Self {
        Self {
            raw: RwLock::new(HashMap::with_capacity(1024))
        }
    }
    
    pub async fn get(&self, a: &Addr) -> Arc<String> {
        if let Some(n) = self.raw.read().await.get(a) {
            if !n.is_timeout() {
                return n.unwind()   
            }
        }

        self.raw
            .write()
            .await
            .entry(a.clone())
            .or_insert_with(Nonce::new)
            .unwind()
    }

    pub async fn remove(&self, a: &Addr) {
        self.raw.write().await.remove(a);
    }
}

impl Nonce {
    pub fn new() -> Self {
        Self {
            raw: Arc::new(nonce()),
            timer: Instant::now()
        }
    }

    pub fn is_timeout(&self) -> bool {
        self.timer.elapsed().as_secs() < 3600
    }

    pub fn unwind(&self) -> Arc<String> {
        self.raw.clone()
    }
}


fn nonce() -> String {
    let mut rng = thread_rng();
    let r = std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect::<String>();
    r.to_lowercase()
}
