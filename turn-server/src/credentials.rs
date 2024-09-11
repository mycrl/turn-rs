use std::{
    sync::{Arc, RwLock},
    thread::{self, sleep},
    time::{Duration, Instant},
};

use ahash::HashMap;

pub struct StaticPassword {
    pub value: String,
    lifetime: Option<Instant>,
}

#[derive(Clone)]
pub struct StaticCredentials(Arc<RwLock<HashMap<String, StaticPassword>>>);

impl From<std::collections::HashMap<String, String>> for StaticCredentials {
    fn from(value: std::collections::HashMap<String, String>) -> Self {
        let this = Self::new();

        for (k, v) in value {
            this.set(k, v, true);
        }

        this
    }
}

impl AsRef<Arc<RwLock<HashMap<String, StaticPassword>>>> for StaticCredentials {
    fn as_ref(&self) -> &Arc<RwLock<HashMap<String, StaticPassword>>> {
        &self.0
    }
}

impl StaticCredentials {
    pub fn new() -> Self {
        let map: Arc<RwLock<HashMap<String, StaticPassword>>> = Default::default();

        let map_ = Arc::downgrade(&map);
        thread::spawn(move || {
            let mut keys_ = Vec::new();

            while let Some(map) = map_.upgrade() {
                keys_.clear();

                {
                    for (key, value) in map.read().unwrap().iter() {
                        if let Some(lifetime) = value.lifetime {
                            if lifetime.elapsed().as_secs() >= 86400 {
                                keys_.push(key.clone());
                            }
                        }
                    }

                    let mut map_ = map.write().unwrap();
                    for key in &keys_ {
                        map_.remove(key);
                    }
                }

                sleep(Duration::from_secs(60));
            }
        });

        Self(map)
    }

    pub fn set(&self, username: String, password: String, permanent: bool) {
        self.0.write().unwrap().insert(
            username,
            StaticPassword {
                value: password,
                lifetime: if !permanent {
                    Some(Instant::now())
                } else {
                    None
                },
            },
        );
    }
}
