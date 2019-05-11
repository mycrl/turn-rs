// use.
use bytes::Bytes;
use std::collections::HashMap;
use std::collections::VecDeque;
use crate::CONFIGURE;


/// # Matedata Bytes.
#[derive(Clone)]
pub struct CacheBytes {
    pub audio: Option<Bytes>,
    pub video: Option<Bytes>
}


/// # Chip Frame Pool.
#[derive(Clone)]
pub struct BytesPool {
    pub pool: VecDeque<CacheBytes>,
    pub len: usize
}


/// # Live Information.
#[derive(Clone)]
pub struct Live {
    pub name: String,
    pub key: String,
    pub bytes: BytesPool
}


/// # Live Poll And Connection Poll.
pub struct Pool {
    pub lives: HashMap<String, Live>,
    pub max: u8
}


impl BytesPool {

    /// # Create bytes pool.
    pub fn new (max: usize) -> Self {
        BytesPool {
            pool: VecDeque::with_capacity(max),
            len: max
        }
    }

    /// # Append value in bytes pool.
    /// /// When appending, it will first check if it will exceed the boundary.
    /// If the boundary is exceeded, the first bit is cleared first.
    pub fn append (&mut self, bytes: CacheBytes) {
        if self.pool.len() >= self.len {
            self.pool.remove(1);
        }

        // append to buffer.
        self.pool.push_back(bytes);
    }

 
    /// # Get the first element in the buffer pool.
    /// When fetching, the currently fetched value is cleared from the buffer.
    pub fn get (&mut self) -> Option<CacheBytes> {
        self.pool.remove(1)
    }
}


impl Pool {

    /// # Created pool.
    pub fn new () -> Self {
        Pool { 
            lives: HashMap::new(),
            max: CONFIGURE.pool.bytes
        }
    }

    /// # Create new matedata pool.
    pub fn create (&mut self, name: String, key: String) {
        let bytes = BytesPool::new(self.max as usize);
        let live = Live { name: name.clone(), key, bytes };
        self.lives.insert(name, live);
    }

    /// # Put live.
    /// Put the audio and video streams into the streaming media pool.
    /// If the channel already exists, it will no longer be created.
    pub fn put (&mut self, name: String, key: String, bytes: CacheBytes) {
        match self.lives.get_mut(&name) {
            Some(live) => live.bytes.append(bytes),
            None => self.create(name, key)
        }
    }

    /// # Get live.
    pub fn read (&mut self, name: String, key: String) -> Option<CacheBytes> {
        let mut value = None;

        // check if the push stream key matches.
        if let Some(live) = self.lives.get_mut(&name) {
            if &live.key == &key {
                value = live.bytes.get();
            }
        }

        value
    }
}