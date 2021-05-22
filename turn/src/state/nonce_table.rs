use super::Addr;
use std::{
    collections::HashMap,
    sync::Arc
};

use tokio::{
    time::Instant,
    sync::RwLock
};

use rand::{
    distributions::Alphanumeric, 
    thread_rng, 
    Rng
};

/// Session nonce.
///
/// The NONCE attribute may be present in requests and responses.  It
/// contains a sequence of qdtext or quoted-pair, which are defined in
/// [RFC3261](https://datatracker.ietf.org/doc/html/rfc3261).  
/// Note that this means that the NONCE attribute will not
/// contain the actual surrounding quote characters.  The NONCE attribute
/// MUST be fewer than 128 characters (which can be as long as 509 bytes
/// when encoding them and a long as 763 bytes when decoding them).  See
/// Section 5.4 of [RFC7616](https://datatracker.ietf.org/doc/html/rfc7616#section-5.4) 
/// for guidance on selection of nonce values in a server.
pub struct Nonce {
    raw: Arc<String>,
    timer: Instant
}

/// Nonce table.
pub struct NonceTable {
    raw: RwLock<HashMap<Addr, Nonce>>
}

impl NonceTable {
    pub fn new() -> Self {
        Self {
            raw: RwLock::new(HashMap::with_capacity(1024))
        }
    }
    
    /// get session nonce string.
    ///
    /// each node is assigned a random string valid for 1 hour.
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// 
    /// let addr = "127.0.0.1:1080".parse::<SocketAddr>().unwrap(); 
    /// let nonce_table = NonceTable::new();
    /// // nonce_table.get(&addr)
    /// ```
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

    /// remove session nonce string.
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// 
    /// let addr = "127.0.0.1:1080".parse::<SocketAddr>().unwrap(); 
    /// let nonce_table = NonceTable::new();
    /// // nonce_table.get(&addr);
    /// nonce_table.remove(&addr);
    /// ```
    pub async fn remove(&self, a: &Addr) {
        self.raw.write().await.remove(a);
    }
}

impl Nonce {
    pub fn new() -> Self {
        Self {
            raw: Arc::new(Self::create_nonce()),
            timer: Instant::now()
        }
    }

    /// whether the nonce is dead.
    ///
    /// ```no_run
    /// let node = Nonce::new();
    /// assert!(!node.is_death());
    /// ```
    pub fn is_death(&self) -> bool {
        self.timer.elapsed().as_secs() < 3600
    }

    /// create node session.
    ///
    /// node session from group number and long key.
    ///
    /// ```no_run
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// // Node::new(0, key.clone());
    /// ```
    pub fn unwind(&self) -> Arc<String> {
        self.raw.clone()
    }
    
    /// generate nonce string.
    fn create_nonce() -> String {
        let mut rng = thread_rng();
        std::iter::repeat(())
            .map(|_| rng.sample(Alphanumeric))
            .take(16)
            .collect::<String>()
            .to_lowercase()
    }
}
