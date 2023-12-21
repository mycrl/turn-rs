use super::ports::capacity;

use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Instant,
};

use ahash::AHashMap;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

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
    timer: Instant,
}

impl Default for Nonce {
    fn default() -> Self {
        Self::new()
    }
}

impl Nonce {
    pub fn new() -> Self {
        Self {
            raw: Arc::new(Self::create_nonce()),
            timer: Instant::now(),
        }
    }

    /// whether the nonce is dead.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nonces::*;
    ///
    /// let nonce = Nonce::new();
    /// assert!(!nonce.is_death());
    /// ```
    pub fn is_death(&self) -> bool {
        self.timer.elapsed().as_secs() >= 3600
    }

    /// unwind nonce random string.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nonces::*;
    ///
    /// let nonce = Nonce::new();
    /// assert_eq!(nonce.unwind().len(), 16);
    /// ```
    pub fn unwind(&self) -> Arc<String> {
        self.raw.clone()
    }

    /// generate nonce string.
    fn create_nonce() -> String {
        let mut rng = thread_rng();
        std::iter::repeat(())
            .map(|_| rng.sample(Alphanumeric) as char)
            .take(16)
            .collect::<String>()
            .to_lowercase()
    }
}

/// nonce table.
pub struct Nonces {
    map: RwLock<AHashMap<SocketAddr, Nonce>>,
}

impl Default for Nonces {
    fn default() -> Self {
        Self::new()
    }
}

impl Nonces {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(AHashMap::with_capacity(capacity())),
        }
    }

    /// get session nonce string.
    ///
    /// each node is assigned a random string valid for 1 hour.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nonces::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:1080".parse::<SocketAddr>().unwrap();
    /// let nonce_table = Nonces::new();
    ///
    /// assert_eq!(nonce_table.get(&addr).len(), 16);
    /// ```
    pub fn get(&self, a: &SocketAddr) -> Arc<String> {
        if let Some(n) = self.map.read().unwrap().get(a) {
            if !n.is_death() {
                return n.unwind();
            }
        }

        self.map
            .write()
            .unwrap()
            .entry(*a)
            .or_insert_with(Nonce::new)
            .unwind()
    }

    /// remove session nonce string.
    ///
    /// # Examples
    ///
    /// ```
    /// use turn_rs::router::nonces::*;
    /// use std::net::SocketAddr;
    ///
    /// let addr = "127.0.0.1:1080".parse::<SocketAddr>().unwrap();
    /// let nonce_table = Nonces::new();
    ///
    /// let nonce = nonce_table.get(&addr);
    /// nonce_table.remove(&addr);
    ///
    /// let new_nonce = nonce_table.get(&addr);
    /// assert!(nonce.as_str() != new_nonce.as_str());
    /// ```
    pub fn remove(&self, a: &SocketAddr) {
        self.map.write().unwrap().remove(a);
    }
}
