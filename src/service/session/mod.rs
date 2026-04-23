pub mod ports;

use super::{
    ServiceHandler, Transport,
    session::ports::{PortAllocator, PortRange},
};

use crate::codec::{crypto::Password, message::attributes::PasswordAlgorithm};

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
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use rand::{Rng, distr::Alphanumeric};

/// The identifier of the session.
///
/// Each session needs to be identified by a combination of three pieces of
/// information: the source address, and the transport protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub source: SocketAddr,
    pub external: SocketAddr,
    pub interface: SocketAddr,
    pub transport: Transport,
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
/// use turn_server::service::session::Timer;
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

/// Default session lifetime in seconds (10 minutes)
pub const DEFAULT_SESSION_LIFETIME: u64 = 600;

/// turn session information.
///
/// A user can have many sessions.
///
/// The default survival time for a session is 600 seconds.
#[derive(Debug, Clone)]
pub enum Session {
    New {
        nonce: String,
        expires: u64,
    },
    Authenticated {
        nonce: String,
        /// Authentication information for the session.
        ///
        /// Digest data is data that summarises usernames and passwords by means of
        /// long-term authentication.
        username: String,
        password: Password,
        /// Assignment information for the session.
        ///
        /// SessionManager are all bound to only one port and one channel.
        allocated_port: Option<u16>,
        // Stores the address to which the session should be forwarded when it sends indication to a
        // port. This is written when permissions are created to allow a certain address to be
        // forwarded to the current session.
        port_relay_table: HashMap</* port */ u16, Identifier>,
        // Indicates to which session the data sent by a session to a channel should be forwarded.
        channel_relay_table: HashMap</* channel */ u16, Identifier>,
        expires: u64,
    },
}

impl Session {
    /// Get the nonce of the session.
    pub fn nonce(&self) -> &str {
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
            port_mapping_table: RwLock::new(Table::default()),
            sessions: RwLock::new(Table::default()),
            timer: Timer::default(),
            handler: options.handler,
        });

        // This is a background thread that silently handles expiring sessions and
        // cleans up session information when it expires.
        let this_ = Arc::downgrade(&this);
        thread::spawn(move || {
            let mut identifiers = Vec::with_capacity(255);

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
                            .for_each(|(k, _)| identifiers.push(*k));
                    }

                    // Delete the expired sessions.
                    if !identifiers.is_empty() {
                        this.remove_session(&identifiers);
                        identifiers.clear();
                    }
                }

                // Fixing a second tick.
                sleep(Duration::from_secs(1));
            }
        });

        this
    }

    fn remove_session(&self, identifiers: &[Identifier]) {
        let mut sessions = self.sessions.write();
        let mut port_allocator = self.port_allocator.lock();
        let mut port_mapping_table = self.port_mapping_table.write();

        identifiers.iter().for_each(|k| {
            if let Some(Session::Authenticated {
                allocated_port,
                username,
                ..
            }) = sessions.remove(k)
            {
                // Removes the session-bound port from the port binding table and
                // releases the port back into the allocation pool.
                if let Some(port) = allocated_port {
                    port_mapping_table.remove(&port);
                    port_allocator.deallocate(port);
                }

                // Notifies that the external session has been closed.
                self.handler.on_destroy(k, &username);
            }
        });
    }

    /// Get session for identifier.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::service::session::*;
    /// use turn_server::service::*;
    /// use turn_server::codec::message::attributes::PasswordAlgorithm;
    /// use turn_server::codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let identifier = Identifier::new(
    ///     "127.0.0.1:8080".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
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
    ///     assert!(sessions.get_session(&identifier).get_ref().is_none());
    /// }
    ///
    /// // get_session always creates a new session if it doesn't exist
    /// {
    ///     let lock = sessions.get_session_or_default(&identifier);
    ///     let session = lock.get_ref().unwrap();
    ///     match session {
    ///         Session::New { .. } => {},
    ///         _ => panic!("Expected new session"),
    ///     }
    /// }
    ///
    /// sessions.get_password(&identifier, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// {
    ///     let lock = sessions.get_session(&identifier);
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
                return ReadLock { lock, key };
            }
        }

        {
            self.sessions.write().insert(
                *key,
                Session::New {
                    // A random string of length 16.
                    nonce: generate_nonce(),
                    // Current time stacks for DEFAULT_SESSION_LIFETIME seconds.
                    expires: self.timer.get() + DEFAULT_SESSION_LIFETIME,
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

    /// Get digest for identifier.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::service::session::*;
    /// use turn_server::service::*;
    /// use turn_server::codec::message::attributes::PasswordAlgorithm;
    /// use turn_server::codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let identifier = Identifier::new(
    ///     "127.0.0.1:8080".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
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
    ///     sessions.get_session(&identifier);
    /// }
    /// assert_eq!(pollster::block_on(sessions.get_password(&identifier, "test1", PasswordAlgorithm::Md5)), None);
    ///
    /// // Create a new session for the next test
    /// {
    ///     sessions.get_session(&identifier);
    /// }
    /// assert_eq!(sessions.get_password(&identifier, "test", PasswordAlgorithm::Md5).block_on(), Some(digest));
    ///
    /// // The third call should return cached digest
    /// assert_eq!(sessions.get_password(&identifier, "test", PasswordAlgorithm::Md5).block_on(), Some(digest));
    /// ```
    pub async fn get_password(
        &self,
        identifier: &Identifier,
        username: &str,
        algorithm: PasswordAlgorithm,
    ) -> Option<Password> {
        // Already authenticated, get the cached digest directly.
        {
            if let Some(Session::Authenticated { password, .. }) =
                self.sessions.read().get(identifier)
            {
                return Some(*password);
            }
        }

        // Get the current user's password from an external handler and create a
        // digest.
        let password = self
            .handler
            .get_password(identifier, username, algorithm)
            .await?;

        // Record a new session.
        {
            let mut lock = self.sessions.write();

            let nonce = if let Some(Session::New { nonce, .. }) = lock.remove(identifier) {
                nonce
            } else {
                generate_nonce()
            };

            lock.insert(
                *identifier,
                Session::Authenticated {
                    port_relay_table: HashMap::with_capacity(10),
                    channel_relay_table: HashMap::with_capacity(10),
                    expires: self.timer.get() + DEFAULT_SESSION_LIFETIME,
                    username: username.to_string(),
                    allocated_port: None,
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
    /// use turn_server::service::session::*;
    /// use turn_server::service::*;
    /// use turn_server::codec::message::attributes::PasswordAlgorithm;
    /// use turn_server::codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let identifier = Identifier::new(
    ///     "127.0.0.1:8080".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
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
    /// sessions.get_password(&identifier, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// {
    ///     let lock = sessions.get_session(&identifier);
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
    /// let port = sessions.allocate(&identifier, None).unwrap();
    /// {
    ///     let lock = sessions.get_session(&identifier);
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
    /// assert_eq!(sessions.allocate(&identifier, None), Some(port));
    /// ```
    pub fn allocate(&self, identifier: &Identifier, lifetime: Option<u32>) -> Option<u16> {
        let mut lock = self.sessions.write();

        if let Some(Session::Authenticated {
            allocated_port,
            expires,
            ..
        }) = lock.get_mut(identifier)
        {
            // If the port has already been allocated, re-allocation is not allowed.
            if let Some(port) = allocated_port {
                return Some(*port);
            }

            // Records the port assigned to the current session and resets the alive time.
            let port = self.port_allocator.lock().allocate(None)?;
            *allocated_port = Some(port);
            *expires =
                self.timer.get() + (lifetime.unwrap_or(DEFAULT_SESSION_LIFETIME as u32) as u64);

            // Write the allocation port binding table.
            self.port_mapping_table.write().insert(port, *identifier);
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
    /// use turn_server::service::session::*;
    /// use turn_server::service::*;
    /// use turn_server::codec::message::attributes::PasswordAlgorithm;
    /// use turn_server::codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let identifier = Identifier::new(
    ///     "127.0.0.1:8080".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
    ///
    /// let peer_identifier = Identifier::new(
    ///     "127.0.0.1:8081".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
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
    /// sessions.get_password(&identifier, "test", PasswordAlgorithm::Md5).block_on();
    /// sessions.get_password(&peer_identifier, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// let port = sessions.allocate(&identifier, None).unwrap();
    /// let peer_port = sessions.allocate(&peer_identifier, None).unwrap();
    ///
    /// assert!(!sessions.create_permission(&identifier, &endpoint, &[port]));
    /// assert!(sessions.create_permission(&identifier, &endpoint, &[peer_port]));
    ///
    /// assert!(!sessions.create_permission(&peer_identifier, &endpoint, &[peer_port]));
    /// assert!(sessions.create_permission(&peer_identifier, &endpoint, &[port]));
    /// ```
    pub fn create_permission(&self, identifier: &Identifier, ports: &[u16]) -> bool {
        // Finds information about the current session.
        if let Some(Session::Authenticated {
            allocated_port,
            port_relay_table,
            ..
        }) = self.sessions.write().get_mut(identifier)
        {
            // The port number assigned to the current session.
            let Some(local_port) = *allocated_port else {
                return false;
            };

            // You cannot create permissions for yourself.
            if ports.contains(&local_port) {
                return false;
            }

            for port in ports {
                if let Some(peer) = self.port_mapping_table.read().get(port) {
                    // Check if the current port is already occupied by another client.
                    if let Some(relay) = port_relay_table.get(&port)
                        && relay != peer
                    {
                        return false;
                    }

                    port_relay_table.insert(*port, *peer);
                } else {
                    return false;
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
    /// use turn_server::service::session::*;
    /// use turn_server::service::*;
    /// use turn_server::codec::message::attributes::PasswordAlgorithm;
    /// use turn_server::codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let identifier = Identifier::new(
    ///     "127.0.0.1:8080".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
    ///
    /// let peer_identifier = Identifier::new(
    ///     "127.0.0.1:8081".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
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
    /// sessions.get_password(&identifier, "test", PasswordAlgorithm::Md5).block_on();
    /// sessions.get_password(&peer_identifier, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// let port = sessions.allocate(&identifier, None).unwrap();
    /// let peer_port = sessions.allocate(&peer_identifier, None).unwrap();
    /// {
    ///     assert_eq!(
    ///         match sessions.get_session(&identifier).get_ref().unwrap() {
    ///             Session::Authenticated { allocate_channels, .. } => allocate_channels.len(),
    ///             _ => panic!("Expected authenticated session"),
    ///         },
    ///         0
    ///     );
    /// }
    ///
    /// {
    ///     assert_eq!(
    ///         match sessions.get_session(&peer_identifier).get_ref().unwrap() {
    ///             Session::Authenticated { allocate_channels, .. } => allocate_channels.len(),
    ///             _ => panic!("Expected authenticated session"),
    ///         },
    ///         0
    ///     );
    /// }
    ///
    /// assert!(sessions.bind_channel(&identifier, &endpoint, peer_port, 0x4000));
    /// assert!(sessions.bind_channel(&peer_identifier, &endpoint, port, 0x4000));
    ///
    /// {
    ///     assert_eq!(
    ///         match sessions.get_session(&identifier).get_ref().unwrap() {
    ///             Session::Authenticated { allocate_channels, .. } => allocate_channels.clone(),
    ///             _ => panic!("Expected authenticated session"),
    ///         },
    ///         vec![0x4000]
    ///     );
    /// }
    ///
    /// {
    ///     assert_eq!(
    ///         match sessions.get_session(&peer_identifier).get_ref().unwrap() {
    ///             Session::Authenticated { allocate_channels, .. } => allocate_channels.clone(),
    ///             _ => panic!("Expected authenticated session"),
    ///         },
    ///         vec![0x4000]
    ///     );
    /// }
    /// ```
    pub fn bind_channel(&self, identifier: &Identifier, port: u16, channel: u16) -> bool {
        // Records the channel used for the current session.
        {
            if let Some(Session::Authenticated {
                channel_relay_table,
                ..
            }) = self.sessions.write().get_mut(identifier)
            {
                // Finds the address of the bound opposing port.
                if let Some(peer) = self.port_mapping_table.read().get(&port) {
                    // Check if the current channel is already occupied by another client.
                    if let Some(relay) = channel_relay_table.get(&channel)
                        && relay != peer
                    {
                        return false;
                    }

                    // Create channel forwarding mapping relationships for peers.
                    channel_relay_table.insert(channel, *peer);
                } else {
                    return false;
                };
            } else {
                return false;
            };
        }

        // Binding ports also creates permissions.
        if !self.create_permission(identifier, &[port]) {
            return false;
        }

        true
    }

    /// Gets the peer of the current session bound channel.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::service::session::*;
    /// use turn_server::service::*;
    /// use turn_server::codec::message::attributes::PasswordAlgorithm;
    /// use turn_server::codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let identifier = Identifier::new(
    ///     "127.0.0.1:8080".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
    ///
    /// let peer_identifier = Identifier::new(
    ///     "127.0.0.1:8081".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
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
    /// sessions.get_password(&identifier, "test", PasswordAlgorithm::Md5).block_on();
    /// sessions.get_password(&peer_identifier, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// let port = sessions.allocate(&identifier, None).unwrap();
    /// let peer_port = sessions.allocate(&peer_identifier, None).unwrap();
    ///
    /// assert!(sessions.bind_channel(&identifier, &endpoint, peer_port, 0x4000));
    /// assert!(sessions.bind_channel(&peer_identifier, &endpoint, port, 0x4000));
    /// assert_eq!(
    ///     sessions
    ///         .get_channel_relay_address(&identifier, 0x4000)
    ///         .unwrap()
    ///         .endpoint(),
    ///     endpoint
    /// );
    ///
    /// assert_eq!(
    ///     sessions
    ///         .get_channel_relay_address(&peer_identifier, 0x4000)
    ///         .unwrap()
    ///         .endpoint(),
    ///     endpoint
    /// );
    /// ```
    pub fn get_channel_relay_address(
        &self,
        identifier: &Identifier,
        channel: u16,
    ) -> Option<(/* peer channel */ u16, Identifier)> {
        let session = self.sessions.read();

        if let Session::Authenticated {
            channel_relay_table,
            ..
        } = session.get(identifier)?
        {
            let peer = channel_relay_table.get(&channel)?;

            if let Session::Authenticated {
                channel_relay_table,
                ..
            } = session.get(peer)?
            {
                // Find the channel bound to the current client.
                let (peer_channel, _) =
                    channel_relay_table.iter().find(|(_, v)| *v == identifier)?;

                Some((*peer_channel, *peer))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get the address of the port binding.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::service::session::*;
    /// use turn_server::service::*;
    /// use turn_server::codec::message::attributes::PasswordAlgorithm;
    /// use turn_server::codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let identifier = Identifier::new(
    ///     "127.0.0.1:8080".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
    ///
    /// let peer_identifier = Identifier::new(
    ///     "127.0.0.1:8081".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
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
    /// sessions.get_password(&identifier, "test", PasswordAlgorithm::Md5).block_on();
    /// sessions.get_password(&peer_identifier, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// let port = sessions.allocate(&identifier, None).unwrap();
    /// let peer_port = sessions.allocate(&peer_identifier, None).unwrap();
    ///
    /// assert!(sessions.create_permission(&identifier, &endpoint, &[peer_port]));
    /// assert!(sessions.create_permission(&peer_identifier, &endpoint, &[port]));
    ///
    /// assert_eq!(
    ///     sessions
    ///         .get_port_relay_address(&identifier, peer_port)
    ///         .unwrap()
    ///         .endpoint(),
    ///     endpoint
    /// );
    ///
    /// assert_eq!(
    ///     sessions
    ///         .get_relay_address(&peer_identifier, port)
    ///         .unwrap()
    ///         .endpoint(),
    ///     endpoint
    /// );
    /// ```
    pub fn get_port_relay_address(
        &self,
        identifier: &Identifier,
        port: u16,
    ) -> Option<(/* local port */ u16, Identifier)> {
        if let Session::Authenticated {
            port_relay_table,
            allocated_port,
            ..
        } = self.sessions.read().get(identifier)?
        {
            Some(((*allocated_port)?, port_relay_table.get(&port).copied()?))
        } else {
            None
        }
    }

    /// Refresh the session for identifier.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::service::session::*;
    /// use turn_server::service::*;
    /// use turn_server::codec::message::attributes::PasswordAlgorithm;
    /// use turn_server::codec::crypto::Password;
    /// use pollster::FutureExt;
    ///
    /// #[derive(Clone)]
    /// struct ServiceHandlerTest;
    ///
    /// impl ServiceHandler for ServiceHandlerTest {
    ///     async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let identifier = Identifier::new(
    ///     "127.0.0.1:8080".parse().unwrap(),
    ///     "127.0.0.1:3478".parse().unwrap(),
    /// );
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
    ///     let lock = sessions.get_session_or_default(&identifier);
    ///     let session = lock.get_ref().unwrap();
    ///     match session {
    ///         Session::New { .. } => {},
    ///         _ => panic!("Expected new session"),
    ///     }
    /// }
    ///
    /// sessions.get_password(&identifier, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// {
    ///     let lock = sessions.get_session(&identifier);
    ///     let expires = match lock.get_ref().unwrap() {
    ///         Session::Authenticated { expires, .. } => *expires,
    ///         _ => panic!("Expected authenticated session"),
    ///     };
    ///
    ///     assert!(expires == 600 || expires == 601 || expires == 602);
    /// }
    ///
    /// assert!(sessions.refresh(&identifier, 0));
    ///
    /// // After refresh with lifetime 0, session should be removed
    /// {
    ///     assert!(sessions.get_session(&identifier).get_ref().is_none());
    /// }
    /// ```
    pub fn refresh(&self, identifier: &Identifier, lifetime: u32) -> bool {
        if lifetime > 3600 {
            return false;
        }

        if lifetime == 0 {
            self.remove_session(&[*identifier]);
        } else if let Some(Session::Authenticated { expires, .. }) =
            self.sessions.write().get_mut(identifier)
        {
            *expires = self.timer.get() + lifetime as u64;
        } else {
            return false;
        }

        true
    }
}

/// Generate a cryptographically random nonce for STUN/TURN authentication.
///
/// The nonce is a critical security component in STUN/TURN's long-term credential
/// mechanism (RFC 5389). It serves multiple purposes:
/// - Prevents replay attacks by ensuring each authentication is unique
/// - Acts as a server-issued challenge in the digest authentication flow
/// - Binds authentication attempts to specific sessions
///
/// This implementation generates a 16-character alphanumeric string using a
/// cryptographically secure random number generator. The length is chosen to
/// provide sufficient entropy (approximately 95 bits) to make brute-force
/// attacks computationally infeasible while remaining well under the RFC's
/// 128-character limit.
///
/// # Returns
/// A random 16-character string containing alphanumeric characters [a-zA-Z0-9].
///
/// # Security
/// Uses `rand::rng()` which provides cryptographic-quality randomness suitable
/// for security-sensitive operations. See RFC 7616 Section 5.4 for additional
/// guidance on nonce value selection.
fn generate_nonce() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}
