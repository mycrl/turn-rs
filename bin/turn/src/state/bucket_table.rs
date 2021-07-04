use super::random_port::RandomPort;
use std::collections::HashMap;
use tokio::sync::Mutex;

/// group namespace.
///
/// The group contains a reference count, 
/// when there is no reference, it can be 
/// deleted, and the port uses a random 
/// allocation algorithm.
pub struct Bucket {
    num: usize,
    port: RandomPort
}

/// buckets table.
pub struct BucketTable {
    raw: Mutex<HashMap<u32, Bucket>>
}

impl BucketTable {
    pub fn new() -> Self {
        Self {
            raw: Mutex::new(HashMap::with_capacity(100))
        }
    }
    
    /// allocate a port to the bucket.
    /// 
    /// ```no_run
    /// let buckets = BucketTable::new();
    /// // buckets.alloc(0).await.is_some()
    /// ```
    pub async fn alloc(&self, group: u32) -> Option<u16> {
        self.raw
            .lock()
            .await
            .entry(group)
            .or_insert_with(Bucket::new)
            .alloc()
    }

    /// remove an allocated from the bucket.
    /// 
    /// ```no_run
    /// let buckets = BucketTable::new();
    /// let port = buckets.alloc(0).await.unwrap();
    /// // buckets.remove(0, port).await
    /// ```
    pub async fn remove(&self, group: u32, port: u16) {
        let mut inner = self.raw.lock().await;
        if let Some(bucket) = inner.get_mut(&group) {
            bucket.remove(port);
            if bucket.num == 0 {
                inner.remove(&group);
            }
        }
    }
}

impl Bucket {
    /// use random port allocation algorithm 
    /// to allocate in the range of 49152-65535.
    pub fn new() -> Self {
        Self {
            port: RandomPort::new(49152..65535),
            num: 0,
        }
    }

    /// allocated a port to the bucket.
    ///
    /// if the allocation is successful, 
    /// add the reference count.
    /// 
    /// ```no_run
    /// let mut bucket = Bucket::new();
    /// // bucket.alloc(0).is_some()
    /// ```
    pub fn alloc(&mut self) -> Option<u16> {
        let port = self.port.alloc(None);
        if port.is_some() {
            self.num += 1;
        }

        port
    }

    /// remove an allocated from the bucket.
    ///
    /// if the remove is successful, 
    /// subtract the reference count.
    /// 
    /// ```no_run
    /// let mut bucket = Bucket::new();
    /// let port = bucket.alloc(0).unwrap();
    /// // bucket.remove(0, port)
    /// ```
    pub fn remove(&mut self, port: u16) {
        self.port.restore(port);
        self.num -= 1;
    }
}
