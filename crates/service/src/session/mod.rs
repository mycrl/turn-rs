pub mod ports;

use crate::{
    ServiceHandler,
    session::ports::{PortAllocator, PortRange},
};

use std::{
    hash::Hash,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, sleep},
    time::Duration,
};

use ahash::{HashMap, HashMapExt};
use codec::{crypto::Password, message::attributes::PasswordAlgorithm};
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use rand::Rng;

/// The identifier of the session or addr.
///
/// Each session needs to be identified by a combination of three pieces of
/// information: the addr address, and the transport protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub source: SocketAddr,
    pub interface: SocketAddr,
}

/// The addr used to record the current session.
///
/// This is used when forwarding data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Endpoint {
    pub source: SocketAddr,
    pub endpoint: SocketAddr,
}

/// The default HashMap is created without allocating capacity. To improve
/// performance, the turn server needs to pre-allocate the available capacity.
///
/// So here the HashMap is rewrapped to allocate a large capacity (number of
/// ports that can be allocated) at the default creation time as well.
pub struct Table<K, V>(HashMap<K, V>);

impl<K, V> Default for Table<K, V> {
    fn default() -> Self {
        Self(HashMap::with_capacity(PortRange::default().size()))
    }
}

impl<K, V> AsRef<HashMap<K, V>> for Table<K, V> {
    fn as_ref(&self) -> &HashMap<K, V> {
        &self.0
    }
}

impl<K, V> Deref for Table<K, V> {
    type Target = HashMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V> DerefMut for Table<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Used to lengthen the timing of the release of a readable lock guard and to
/// provide a more convenient way for external access to the lock's internal
/// data.
pub struct ReadLock<'a, 'b, K, R> {
    pub key: &'a K,
    pub lock: RwLockReadGuard<'b, R>,
}

impl<'a, 'b, K, V> ReadLock<'a, 'b, K, Table<K, V>>
where
    K: Eq + Hash,
{
    pub fn get_ref(&self) -> Option<&V> {
        self.lock.get(self.key)
    }
}

/// A specially optimised timer.
///
/// This timer does not stack automatically and needs to be stacked externally
/// and manually.
///
/// ```
/// use turn_server_service::session::Timer;
///
/// let timer = Timer::default();
///
/// assert_eq!(timer.get(), 0);
/// assert_eq!(timer.add(), 1);
/// assert_eq!(timer.get(), 1);
/// ```
#[derive(Default)]
pub struct Timer(AtomicU64);

impl Timer {
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    pub fn add(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// turn session information.
///
/// A user can have many sessions.
///
/// The default survival time for a session is 600 seconds.
#[derive(Debug, Clone)]
pub enum Session {
    New {
        nonce: [u8; 16],
        expires: u64,
    },
    Authenticated {
        nonce: [u8; 16],
        /// Authentication information for the session.
        ///
        /// Digest data is data that summarises usernames and passwords by means of
        /// long-term authentication.
        username: String,
        password: Password,
        /// Assignment information for the session.
        ///
        /// SessionManager are all bound to only one port and one channel.
        allocate_port: Option<u16>,
        allocate_channels: Vec<u16>,
        permissions: Vec<u16>,
        expires: u64,
    },
}

impl Session {
    /// Get the nonce of the session.
    pub fn nonce(&self) -> &[u8; 16] {
        match self {
            Session::New { nonce, .. } | Session::Authenticated { nonce, .. } => nonce,
        }
    }

    /// Check if the session is a new session.
    pub fn is_new(&self) -> bool {
        matches!(self, Session::New { .. })
    }

    /// Check if the session is an authenticated session.
    pub fn is_authenticated(&self) -> bool {
        matches!(self, Session::Authenticated { .. })
    }
}

pub struct SessionManagerOptions<T> {
    pub port_range: PortRange,
    pub handler: T,
}

pub struct SessionManager<T> {
    sessions: RwLock<Table<Identifier, Session>>,
    port_allocator: Mutex<PortAllocator>,
    // Records the sessions corresponding to each assigned port, which will be needed when looking
    // up sessions assigned to this port based on the port number.
    port_mapping_table: RwLock<Table</* port */ u16, Identifier>>,
    // Stores the address to which the session should be forwarded when it sends indication to a
    // port. This is written when permissions are created to allow a certain address to be
    // forwarded to the current session.
    port_relay_table: RwLock<Table<Identifier, HashMap</* port */ u16, Endpoint>>>,
    // Indicates to which session the data sent by a session to a channel should be forwarded.
    channel_relay_table: RwLock<Table<Identifier, HashMap</* channel */ u16, Endpoint>>>,
    timer: Timer,
    handler: T,
}

impl<T> SessionManager<T>
where
    T: ServiceHandler,
{
    pub fn new(options: SessionManagerOptions<T>) -> Arc<Self> {
        let this = Arc::new(Self {
            port_allocator: Mutex::new(PortAllocator::new(options.port_range)),
            channel_relay_table: RwLock::new(Table::default()),
            port_mapping_table: RwLock::new(Table::default()),
            port_relay_table: RwLock::new(Table::default()),
            sessions: RwLock::new(Table::default()),
            timer: Timer::default(),
            handler: options.handler,
        });

        // This is a background thread that silently handles expiring sessions and
        // cleans up session information when it expires.
        let this_ = Arc::downgrade(&this);
        thread::spawn(move || {
            let mut address = Vec::with_capacity(255);

            while let Some(this) = this_.upgrade() {
                // The timer advances one second and gets the current time offset.
                let now = this.timer.add();

                // This is the part that deletes the session information.
                {
                    // Finds sessions that have expired.
                    {
                        this.sessions
                            .read()
                            .iter()
                            .filter(|(_, v)| match v {
                                Session::New { expires, .. }
                                | Session::Authenticated { expires, .. } => *expires <= now,
                            })
                            .for_each(|(k, _)| address.push(*k));
                    }

                    // Delete the expired sessions.
                    if !address.is_empty() {
                        this.remove_session(&address);
                        address.clear();
                    }
                }

                // Fixing a second tick.
                sleep(Duration::from_secs(1));
            }
        });

        this
    }

    fn remove_session(&self, addrs: &[Identifier]) {
        let mut sessions = self.sessions.write();
        let mut port_allocator = self.port_allocator.lock();
        let mut port_mapping_table = self.port_mapping_table.write();
        let mut port_relay_table = self.port_relay_table.write();
        let mut channel_relay_table = self.channel_relay_table.write();

        addrs.iter().for_each(|k| {
            port_relay_table.remove(k);
            channel_relay_table.remove(k);

            if let Some(Session::Authenticated {
                allocate_port,
                username,
                ..
            }) = sessions.remove(k)
            {
                // Removes the session-bound port from the port binding table and
                // releases the port back into the allocation pool.
                if let Some(port) = allocate_port {
                    port_mapping_table.remove(&port);
                    port_allocator.restore(port);
                }

                // Notifies that the external session has been closed.
                self.handler.on_destroy(k, &username);
            }
        });
    }

    /// Get session for addr.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::*;
    /// use turn_server_service::*;
    /// use codec::message::attributes::PasswordAlgorithm;
    /// use codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = Password::Md5([
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ]);
    ///
    /// let sessions = SessionManager::new(SessionManagerOptions {
    ///     port_range: (49152..65535).into(),
    ///     handler: ServiceHandlerTest,
    /// });
    ///
    /// // get_session always creates a new session if it doesn't exist
    /// {
    ///     assert!(sessions.get_session(&addr).get_ref().is_none());
    /// }
    ///
    /// // get_session always creates a new session if it doesn't exist
    /// {
    ///     let lock = sessions.get_session_or_default(&addr);
    ///     let session = lock.get_ref().unwrap();
    ///     match session {
    ///         Session::New { .. } => {},
    ///         _ => panic!("Expected new session"),
    ///     }
    /// }
    ///
    /// sessions.get_password(&addr, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// {
    ///     let lock = sessions.get_session(&addr);
    ///     let session = lock.get_ref().unwrap();
    ///     match session {
    ///         Session::Authenticated { username, allocate_port, allocate_channels, .. } => {
    ///             assert_eq!(username, "test");
    ///             assert_eq!(allocate_port, &None);
    ///             assert_eq!(allocate_channels.len(), 0);
    ///         }
    ///         _ => panic!("Expected authenticated session"),
    ///     }
    /// }
    /// ```
    pub fn get_session_or_default<'a, 'b>(
        &'a self,
        key: &'b Identifier,
    ) -> ReadLock<'b, 'a, Identifier, Table<Identifier, Session>> {
        {
            let lock = self.sessions.read();
            if lock.contains_key(key) {
                return ReadLock {
                    lock: self.sessions.read(),
                    key,
                };
            }
        }

        {
            self.sessions.write().insert(
                *key,
                Session::New {
                    // A random string of length 16.
                    nonce: make_nonce(),
                    // Current time stacks for 600 seconds.
                    expires: self.timer.get() + 600,
                },
            );
        }

        ReadLock {
            lock: self.sessions.read(),
            key,
        }
    }

    pub fn get_session<'a, 'b>(
        &'a self,
        key: &'b Identifier,
    ) -> ReadLock<'b, 'a, Identifier, Table<Identifier, Session>> {
        ReadLock {
            lock: self.sessions.read(),
            key,
        }
    }

    /// Get digest for addr.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::*;
    /// use turn_server_service::*;
    /// use codec::message::attributes::PasswordAlgorithm;
    /// use codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = Password::Md5([
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ]);
    ///
    /// let sessions = SessionManager::new(SessionManagerOptions {
    ///     port_range: (49152..65535).into(),
    ///     handler: ServiceHandlerTest,
    /// });
    ///
    /// // First call get_session to create a new session
    /// {
    ///     sessions.get_session(&addr);
    /// }
    /// assert_eq!(pollster::block_on(sessions.get_password(&addr, "test1", PasswordAlgorithm::Md5)), None);
    ///
    /// // Create a new session for the next test
    /// {
    ///     sessions.get_session(&addr);
    /// }
    /// assert_eq!(sessions.get_password(&addr, "test", PasswordAlgorithm::Md5).block_on(), Some(digest));
    ///
    /// // The third call should return cached digest
    /// assert_eq!(sessions.get_password(&addr, "test", PasswordAlgorithm::Md5).block_on(), Some(digest));
    /// ```
    pub async fn get_password(
        &self,
        addr: &Identifier,
        username: &str,
        algorithm: PasswordAlgorithm,
    ) -> Option<Password> {
        // Already authenticated, get the cached digest directly.
        {
            if let Some(Session::Authenticated { password, .. }) = self.sessions.read().get(addr) {
                return Some(*password);
            }
        }

        // Get the current user's password from an external handler and create a
        // digest.
        let password = self.handler.get_password(username, algorithm).await?;

        // Record a new session.
        {
            let mut lock = self.sessions.write();
            let nonce = if let Some(Session::New { nonce, .. }) = lock.remove(addr) {
                nonce
            } else {
                make_nonce()
            };

            lock.insert(
                *addr,
                Session::Authenticated {
                    allocate_channels: Vec::with_capacity(10),
                    permissions: Vec::with_capacity(10),
                    expires: self.timer.get() + 600,
                    username: username.to_string(),
                    allocate_port: None,
                    password,
                    nonce,
                },
            );
        }

        Some(password)
    }

    pub fn allocated(&self) -> usize {
        self.port_allocator.lock().len()
    }

    /// Assign a port number to the session.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::*;
    /// use turn_server_service::*;
    /// use codec::message::attributes::PasswordAlgorithm;
    /// use codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = Password::Md5([
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ]);
    ///
    /// let sessions = SessionManager::new(SessionManagerOptions {
    ///     port_range: (49152..65535).into(),
    ///     handler: ServiceHandlerTest,
    /// });
    ///
    /// sessions.get_password(&addr, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// {
    ///     let lock = sessions.get_session(&addr);
    ///     let session = lock.get_ref().unwrap();
    ///     match session {
    ///         Session::Authenticated { username, allocate_port, allocate_channels, .. } => {
    ///             assert_eq!(username, "test");
    ///             assert_eq!(allocate_port, &None);
    ///             assert_eq!(allocate_channels.len(), 0);
    ///         }
    ///         _ => panic!("Expected authenticated session"),
    ///     }
    /// }
    ///
    /// let port = sessions.allocate(&addr).unwrap();
    /// {
    ///     let lock = sessions.get_session(&addr);
    ///     let session = lock.get_ref().unwrap();
    ///     match session {
    ///         Session::Authenticated { username, allocate_port, allocate_channels, .. } => {
    ///             assert_eq!(username, "test");
    ///             assert_eq!(allocate_port, &Some(port));
    ///             assert_eq!(allocate_channels.len(), 0);
    ///         }
    ///         _ => panic!("Expected authenticated session"),
    ///     }
    /// }
    ///
    /// assert_eq!(sessions.allocate(&addr), Some(port));
    /// ```
    pub fn allocate(&self, addr: &Identifier) -> Option<u16> {
        let mut lock = self.sessions.write();

        if let Some(Session::Authenticated {
            allocate_port,
            expires,
            ..
        }) = lock.get_mut(addr)
        {
            // If the port has already been allocated, re-allocation is not allowed.
            if let Some(port) = allocate_port {
                return Some(*port);
            }

            // Records the port assigned to the current session and resets the alive time.
            let port = self.port_allocator.lock().alloc(None)?;
            *expires = self.timer.get() + 600;
            *allocate_port = Some(port);

            // Write the allocation port binding table.
            self.port_mapping_table.write().insert(port, *addr);
            Some(port)
        } else {
            None
        }
    }

    /// Create permission for session.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::*;
    /// use turn_server_service::*;
    /// use codec::message::attributes::PasswordAlgorithm;
    /// use codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let peer_addr = Identifier {
    ///     source: "127.0.0.1:8081".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = Password::Md5([
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ]);
    ///
    /// let sessions = SessionManager::new(SessionManagerOptions {
    ///     port_range: (49152..65535).into(),
    ///     handler: ServiceHandlerTest,
    /// });
    ///
    /// sessions.get_password(&addr, "test", PasswordAlgorithm::Md5).block_on();
    /// sessions.get_password(&peer_addr, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// let port = sessions.allocate(&addr).unwrap();
    /// let peer_port = sessions.allocate(&peer_addr).unwrap();
    ///
    /// assert!(!sessions.create_permission(&addr, &endpoint, &[port]));
    /// assert!(sessions.create_permission(&addr, &endpoint, &[peer_port]));
    ///
    /// assert!(!sessions.create_permission(&peer_addr, &endpoint, &[peer_port]));
    /// assert!(sessions.create_permission(&peer_addr, &endpoint, &[port]));
    /// ```
    pub fn create_permission(
        &self,
        addr: &Identifier,
        endpoint: &SocketAddr,
        ports: &[u16],
    ) -> bool {
        let mut sessions = self.sessions.write();
        let mut port_relay_table = self.port_relay_table.write();
        let port_mapping_table = self.port_mapping_table.read();

        // Finds information about the current session.
        if let Some(Session::Authenticated {
            allocate_port,
            permissions,
            ..
        }) = sessions.get_mut(addr)
        {
            // The port number assigned to the current session.
            let local_port = if let Some(it) = allocate_port {
                *it
            } else {
                return false;
            };

            // You cannot create permissions for yourself.
            if ports.contains(&local_port) {
                return false;
            }

            // Each peer port must be present.
            let mut peers = Vec::with_capacity(15);
            for port in ports {
                if let Some(it) = port_mapping_table.get(&port) {
                    peers.push((it, *port));
                } else {
                    return false;
                }
            }

            // Create a port forwarding mapping relationship for each peer session.
            for (peer, port) in peers {
                port_relay_table
                    .entry(*peer)
                    .or_insert_with(|| HashMap::with_capacity(20))
                    .insert(
                        local_port,
                        Endpoint {
                            source: addr.source,
                            endpoint: *endpoint,
                        },
                    );

                // Do not store the same peer ports to the permission list over and over again.
                if !permissions.contains(&port) {
                    permissions.push(port);
                }
            }

            true
        } else {
            false
        }
    }

    /// Binding a channel to the session.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::*;
    /// use turn_server_service::*;
    /// use codec::message::attributes::PasswordAlgorithm;
    /// use codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let peer_addr = Identifier {
    ///     source: "127.0.0.1:8081".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = Password::Md5([
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ]);
    ///
    /// let sessions = SessionManager::new(SessionManagerOptions {
    ///     port_range: (49152..65535).into(),
    ///     handler: ServiceHandlerTest,
    /// });
    ///
    /// sessions.get_password(&addr, "test", PasswordAlgorithm::Md5).block_on();
    /// sessions.get_password(&peer_addr, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// let port = sessions.allocate(&addr).unwrap();
    /// let peer_port = sessions.allocate(&peer_addr).unwrap();
    /// {
    ///     assert_eq!(
    ///         match sessions.get_session(&addr).get_ref().unwrap() {
    ///             Session::Authenticated { allocate_channels, .. } => allocate_channels.len(),
    ///             _ => panic!("Expected authenticated session"),
    ///         },
    ///         0
    ///     );
    /// }
    ///
    /// {
    ///     assert_eq!(
    ///         match sessions.get_session(&peer_addr).get_ref().unwrap() {
    ///             Session::Authenticated { allocate_channels, .. } => allocate_channels.len(),
    ///             _ => panic!("Expected authenticated session"),
    ///         },
    ///         0
    ///     );
    /// }
    ///
    /// assert!(sessions.bind_channel(&addr, &endpoint, peer_port, 0x4000));
    /// assert!(sessions.bind_channel(&peer_addr, &endpoint, port, 0x4000));
    ///
    /// {
    ///     assert_eq!(
    ///         match sessions.get_session(&addr).get_ref().unwrap() {
    ///             Session::Authenticated { allocate_channels, .. } => allocate_channels.clone(),
    ///             _ => panic!("Expected authenticated session"),
    ///         },
    ///         vec![0x4000]
    ///     );
    /// }
    ///
    /// {
    ///     assert_eq!(
    ///         match sessions.get_session(&peer_addr).get_ref().unwrap() {
    ///             Session::Authenticated { allocate_channels, .. } => allocate_channels.clone(),
    ///             _ => panic!("Expected authenticated session"),
    ///         },
    ///         vec![0x4000]
    ///     );
    /// }
    /// ```
    pub fn bind_channel(
        &self,
        addr: &Identifier,
        endpoint: &SocketAddr,
        port: u16,
        channel: u16,
    ) -> bool {
        // Finds the address of the bound opposing port.
        let peer = if let Some(it) = self.port_mapping_table.read().get(&port) {
            *it
        } else {
            return false;
        };

        // Records the channel used for the current session.
        {
            let mut lock = self.sessions.write();
            if let Some(Session::Authenticated {
                allocate_channels, ..
            }) = lock.get_mut(addr)
            {
                if !allocate_channels.contains(&channel) {
                    allocate_channels.push(channel);
                }
            } else {
                return false;
            };
        }

        // Binding ports also creates permissions.
        if !self.create_permission(addr, endpoint, &[port]) {
            return false;
        }

        // Create channel forwarding mapping relationships for peers.
        self.channel_relay_table
            .write()
            .entry(peer)
            .or_insert_with(|| HashMap::with_capacity(10))
            .insert(
                channel,
                Endpoint {
                    source: addr.source,
                    endpoint: *endpoint,
                },
            );

        true
    }

    /// Gets the peer of the current session bound channel.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::*;
    /// use turn_server_service::*;
    /// use codec::message::attributes::PasswordAlgorithm;
    /// use codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let peer_addr = Identifier {
    ///     source: "127.0.0.1:8081".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = Password::Md5([
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ]);
    ///
    /// let sessions = SessionManager::new(SessionManagerOptions {
    ///     port_range: (49152..65535).into(),
    ///     handler: ServiceHandlerTest,
    /// });
    ///
    /// sessions.get_password(&addr, "test", PasswordAlgorithm::Md5).block_on();
    /// sessions.get_password(&peer_addr, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// let port = sessions.allocate(&addr).unwrap();
    /// let peer_port = sessions.allocate(&peer_addr).unwrap();
    ///
    /// assert!(sessions.bind_channel(&addr, &endpoint, peer_port, 0x4000));
    /// assert!(sessions.bind_channel(&peer_addr, &endpoint, port, 0x4000));
    /// assert_eq!(
    ///     sessions
    ///         .get_channel_relay_address(&addr, 0x4000)
    ///         .unwrap()
    ///         .endpoint,
    ///     endpoint
    /// );
    ///
    /// assert_eq!(
    ///     sessions
    ///         .get_channel_relay_address(&peer_addr, 0x4000)
    ///         .unwrap()
    ///         .endpoint,
    ///     endpoint
    /// );
    /// ```
    pub fn get_channel_relay_address(&self, addr: &Identifier, channel: u16) -> Option<Endpoint> {
        self.channel_relay_table
            .read()
            .get(&addr)?
            .get(&channel)
            .copied()
    }

    /// Get the address of the port binding.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::*;
    /// use turn_server_service::*;
    /// use codec::message::attributes::PasswordAlgorithm;
    /// use codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let peer_addr = Identifier {
    ///     source: "127.0.0.1:8081".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = Password::Md5([
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ]);
    ///
    /// let sessions = SessionManager::new(SessionManagerOptions {
    ///     port_range: (49152..65535).into(),
    ///     handler: ServiceHandlerTest,
    /// });
    ///
    /// sessions.get_password(&addr, "test", PasswordAlgorithm::Md5).block_on();
    /// sessions.get_password(&peer_addr, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// let port = sessions.allocate(&addr).unwrap();
    /// let peer_port = sessions.allocate(&peer_addr).unwrap();
    ///
    /// assert!(sessions.create_permission(&addr, &endpoint, &[peer_port]));
    /// assert!(sessions.create_permission(&peer_addr, &endpoint, &[port]));
    ///
    /// assert_eq!(
    ///     sessions
    ///         .get_relay_address(&addr, peer_port)
    ///         .unwrap()
    ///         .endpoint,
    ///     endpoint
    /// );
    ///
    /// assert_eq!(
    ///     sessions
    ///         .get_relay_address(&peer_addr, port)
    ///         .unwrap()
    ///         .endpoint,
    ///     endpoint
    /// );
    /// ```
    pub fn get_relay_address(&self, addr: &Identifier, port: u16) -> Option<Endpoint> {
        self.port_relay_table.read().get(&addr)?.get(&port).copied()
    }

    /// Refresh the session for addr.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server_service::session::*;
    /// use turn_server_service::*;
    /// use codec::message::attributes::PasswordAlgorithm;
    /// use codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let addr = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = Password::Md5([
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ]);
    ///
    /// let sessions = SessionManager::new(SessionManagerOptions {
    ///     port_range: (49152..65535).into(),
    ///     handler: ServiceHandlerTest,
    /// });
    ///
    /// // get_session always creates a new session if it doesn't exist
    /// {
    ///     let lock = sessions.get_session_or_default(&addr);
    ///     let session = lock.get_ref().unwrap();
    ///     match session {
    ///         Session::New { .. } => {},
    ///         _ => panic!("Expected new session"),
    ///     }
    /// }
    ///
    /// sessions.get_password(&addr, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// {
    ///     let lock = sessions.get_session(&addr);
    ///     let expires = match lock.get_ref().unwrap() {
    ///         Session::Authenticated { expires, .. } => *expires,
    ///         _ => panic!("Expected authenticated session"),
    ///     };
    ///
    ///     assert!(expires == 600 || expires == 601 || expires == 602);
    /// }
    ///
    /// assert!(sessions.refresh(&addr, 0));
    ///
    /// // After refresh with lifetime 0, session should be removed
    /// {
    ///     assert!(sessions.get_session(&addr).get_ref().is_none());
    /// }
    /// ```
    pub fn refresh(&self, addr: &Identifier, lifetime: u32) -> bool {
        if lifetime > 3600 {
            return false;
        }

        if lifetime == 0 {
            self.remove_session(&[*addr]);
        } else {
            if let Some(Session::Authenticated { expires, .. }) =
                self.sessions.write().get_mut(addr)
            {
                *expires = self.timer.get() + lifetime as u64;
            } else {
                return false;
            }
        }

        true
    }
}

/// Generate a random nonce.
fn make_nonce() -> [u8; 16] {
    let mut nonce = [0u8; 16];
    rand::rng().fill(&mut nonce);

    nonce
}
