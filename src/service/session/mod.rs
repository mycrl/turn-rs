pub mod ports;

use super::{
    ServiceHandler, Transport,
    session::ports::{PortAllocator, PortRange},
};

use crate::codec::{crypto::Password, message::attributes::PasswordAlgorithm};

use std::{
    hash::Hash,
    net::{IpAddr, SocketAddr},
    ops::{Deref, DerefMut},
    sync::Arc,
    sync::atomic::{AtomicU64, Ordering},
    thread::{self, sleep},
    time::Duration,
};

use ahash::{HashMap, HashMapExt};
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use rand::{Rng, distr::Alphanumeric};
use tokio::net::UdpSocket;

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
        relay_socket: Option<Arc<UdpSocket>>,
        relay_started: bool,
        permissions: HashMap<IpAddr, u64>,
        channels: HashMap<u16, ChannelBinding>,
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

#[derive(Debug, Clone, Copy)]
pub struct ChannelBinding {
    peer: SocketAddr,
    expires: u64,
}

impl ChannelBinding {
    /// Returns the full peer address associated with this binding.
    pub fn peer(&self) -> SocketAddr {
        self.peer
    }
}

/// RFC 8656 permissions are refreshed for five minutes.
const DEFAULT_PERMISSION_LIFETIME: u64 = 300;
/// RFC 8656 channel bindings are refreshed for ten minutes.
const DEFAULT_CHANNEL_LIFETIME: u64 = 600;

pub struct SessionManager<T> {
    sessions: RwLock<Table<Identifier, Session>>,
    port_allocator: Mutex<PortAllocator>,
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
    ///     async fn get_password(&self, id: &Identifier, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let identifier = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     external: "127.0.0.1:3478".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::Udp,
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
    ///         Session::Authenticated { username, allocated_port, permissions, channels, .. } => {
    ///             assert_eq!(username, "test");
    ///             assert_eq!(allocated_port, &None);
    ///             assert!(permissions.is_empty());
    ///             assert!(channels.is_empty());
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
    ///     async fn get_password(&self, id: &Identifier, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let identifier = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     external: "127.0.0.1:3478".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::Udp,
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
                    relay_socket: None,
                    relay_started: false,
                    permissions: HashMap::with_capacity(10),
                    channels: HashMap::with_capacity(10),
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
    ///     async fn get_password(&self, id: &Identifier, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let identifier = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     external: "127.0.0.1:3478".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::Udp,
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
    /// sessions.get_password(&identifier, "test", PasswordAlgorithm::Md5).block_on();
    ///
    /// {
    ///     let lock = sessions.get_session(&identifier);
    ///     let session = lock.get_ref().unwrap();
    ///     match session {
    ///         Session::Authenticated { username, allocated_port, permissions, channels, .. } => {
    ///             assert_eq!(username, "test");
    ///             assert_eq!(allocated_port, &None);
    ///             assert!(permissions.is_empty());
    ///             assert!(channels.is_empty());
    ///         }
    ///         _ => panic!("Expected authenticated session"),
    ///     }
    /// }
    ///
    /// let runtime = tokio::runtime::Runtime::new().unwrap();
    /// let port = runtime.block_on(async { sessions.allocate(&identifier, None).unwrap() });
    /// {
    ///     let lock = sessions.get_session(&identifier);
    ///     let session = lock.get_ref().unwrap();
    ///     match session {
    ///         Session::Authenticated { username, allocated_port, permissions, channels, .. } => {
    ///             assert_eq!(username, "test");
    ///             assert_eq!(allocated_port, &Some(port));
    ///             assert!(permissions.is_empty());
    ///             assert!(channels.is_empty());
    ///         }
    ///         _ => panic!("Expected authenticated session"),
    ///     }
    /// }
    ///
    /// assert_eq!(sessions.allocate(&identifier, None), Some(port));
    /// ```
    pub fn allocate(&self, identifier: &Identifier, lifetime: Option<u32>) -> Option<u16> {
        if let Some(Session::Authenticated {
            allocated_port: Some(port),
            ..
        }) = self.sessions.read().get(identifier)
        {
            return Some(*port);
        }

        let (port, relay_socket) = loop {
            let port = self.port_allocator.lock().allocate(None)?;
            let address = SocketAddr::new(identifier.interface.ip(), port);
            let socket = match std::net::UdpSocket::bind(address) {
                Ok(socket) => socket,
                Err(_) => {
                    self.port_allocator.lock().deallocate(port);
                    continue;
                }
            };
            if socket.set_nonblocking(true).is_err() {
                self.port_allocator.lock().deallocate(port);
                continue;
            }
            match UdpSocket::from_std(socket) {
                Ok(socket) => break (port, Arc::new(socket)),
                Err(_) => self.port_allocator.lock().deallocate(port),
            }
        };

        let mut sessions = self.sessions.write();
        let Some(Session::Authenticated {
            allocated_port,
            relay_socket: session_socket,
            expires,
            ..
        }) = sessions.get_mut(identifier)
        else {
            self.port_allocator.lock().deallocate(port);
            return None;
        };

        if let Some(existing_port) = allocated_port {
            self.port_allocator.lock().deallocate(port);
            return Some(*existing_port);
        }

        *allocated_port = Some(port);
        *session_socket = Some(relay_socket);
        *expires = self.timer.get() + lifetime.unwrap_or(DEFAULT_SESSION_LIFETIME as u32) as u64;
        Some(port)
    }

    /// Installs or refreshes RFC 8656 IP-scoped permissions for an allocation.
    pub fn create_permission(&self, identifier: &Identifier, peers: &[SocketAddr]) -> bool {
        let now = self.timer.get();
        let mut sessions = self.sessions.write();
        let Some(Session::Authenticated {
            allocated_port,
            permissions,
            ..
        }) = sessions.get_mut(identifier)
        else {
            return false;
        };

        if allocated_port.is_none() {
            return false;
        }

        for peer in peers {
            permissions.insert(peer.ip(), now + DEFAULT_PERMISSION_LIFETIME);
        }
        true
    }

    /// Binds a channel to a peer socket and refreshes its IP permission.
    pub fn bind_channel(&self, identifier: &Identifier, peer: SocketAddr, channel: u16) -> bool {
        let now = self.timer.get();
        let mut sessions = self.sessions.write();
        let Some(Session::Authenticated {
            allocated_port,
            permissions,
            channels,
            ..
        }) = sessions.get_mut(identifier)
        else {
            return false;
        };

        if allocated_port.is_none()
            || channels
                .get(&channel)
                .is_some_and(|binding| binding.peer != peer)
            || channels
                .iter()
                .any(|(number, binding)| *number != channel && binding.peer == peer)
        {
            return false;
        }

        permissions.insert(peer.ip(), now + DEFAULT_PERMISSION_LIFETIME);
        channels.insert(
            channel,
            ChannelBinding {
                peer,
                expires: now + DEFAULT_CHANNEL_LIFETIME,
            },
        );
        true
    }

    /// Returns the peer socket bound to a channel while its binding is live.
    pub(crate) fn channel_peer(&self, identifier: &Identifier, channel: u16) -> Option<SocketAddr> {
        let now = self.timer.get();
        let session = self.sessions.read();
        let Session::Authenticated { channels, .. } = session.get(identifier)? else {
            return None;
        };
        let binding = channels.get(&channel)?;
        (binding.expires > now).then_some(binding.peer)
    }

    /// Returns the allocation relay socket when a live permission authorizes a peer.
    pub(crate) fn relay_to_peer(
        &self,
        identifier: &Identifier,
        peer: SocketAddr,
    ) -> Option<Arc<UdpSocket>> {
        let now = self.timer.get();
        let session = self.sessions.read();
        let Session::Authenticated {
            relay_socket,
            permissions,
            ..
        } = session.get(identifier)?
        else {
            return None;
        };

        (permissions.get(&peer.ip()).copied()? > now)
            .then(|| relay_socket.clone())
            .flatten()
    }

    /// Looks up how an inbound relay datagram should be delivered to its client.
    pub(crate) fn relay_from_peer(
        &self,
        identifier: &Identifier,
        peer: SocketAddr,
    ) -> Option<RelayInbound> {
        let now = self.timer.get();
        let session = self.sessions.read();
        let Session::Authenticated {
            permissions,
            channels,
            ..
        } = session.get(identifier)?
        else {
            return None;
        };

        if permissions.get(&peer.ip()).copied()? <= now {
            return None;
        }

        let channel = channels.iter().find_map(|(number, binding)| {
            (binding.peer == peer && binding.expires > now).then_some(*number)
        });
        Some(RelayInbound { channel })
    }

    /// Reports whether the allocation still owns a relay socket.
    pub(crate) fn relay_socket(&self, identifier: &Identifier) -> Option<Arc<UdpSocket>> {
        let session = self.sessions.read();
        let Session::Authenticated { relay_socket, .. } = session.get(identifier)? else {
            return None;
        };
        relay_socket.clone()
    }

    /// Marks a relay socket as owned by a provider worker and returns it once.
    pub(crate) fn start_relay(&self, identifier: &Identifier) -> Option<Arc<UdpSocket>> {
        let mut sessions = self.sessions.write();
        let Session::Authenticated {
            relay_socket,
            relay_started,
            ..
        } = sessions.get_mut(identifier)?
        else {
            return None;
        };
        if *relay_started {
            return None;
        }
        let socket = relay_socket.clone()?;
        *relay_started = true;
        Some(socket)
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
    ///     async fn get_password(&self, id: &Identifier, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
    ///         if username == "test" {
    ///             Some(turn_server::codec::crypto::generate_password(username, "test", "test", algorithm))
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let identifier = Identifier {
    ///     source: "127.0.0.1:8080".parse().unwrap(),
    ///     external: "127.0.0.1:3478".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    ///     transport: Transport::Udp,
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct RelayInbound {
    pub channel: Option<u16>,
}

#[cfg(test)]
mod relay_state_tests {
    use super::*;
    use crate::service::Transport;
    use tokio::time::{Duration, timeout};

    #[derive(Clone)]
    struct Handler;

    impl ServiceHandler for Handler {
        async fn get_password(
            &self,
            _id: &Identifier,
            _username: &str,
            _algorithm: PasswordAlgorithm,
        ) -> Option<Password> {
            Some(Password::Md5([0; 16]))
        }
    }

    fn id(port: u16) -> Identifier {
        Identifier {
            source: SocketAddr::from(([127, 0, 0, 1], port)),
            interface: SocketAddr::from(([127, 0, 0, 1], 3478)),
            external: SocketAddr::from(([127, 0, 0, 1], 3478)),
            transport: Transport::Udp,
        }
    }

    #[tokio::test]
    async fn arbitrary_peer_permission_is_ip_scoped() {
        let manager = SessionManager::new(SessionManagerOptions {
            port_range: (40000..40100).into(),
            handler: Handler,
        });
        let client = id(50000);
        manager
            .get_password(&client, "user", PasswordAlgorithm::Md5)
            .await;
        manager
            .allocate(&client, None)
            .expect("allocation should succeed");
        let peer = SocketAddr::from(([127, 0, 0, 1], 59999));

        assert!(manager.create_permission(&client, &[peer]));
        assert!(manager.relay_to_peer(&client, peer).is_some());
        assert!(
            manager
                .relay_to_peer(&client, SocketAddr::from(([127, 0, 0, 1], 1)))
                .is_some()
        );
    }

    #[tokio::test]
    async fn permitted_arbitrary_udp_peer_exchanges_datagrams_with_relay_socket() {
        let manager = SessionManager::new(SessionManagerOptions {
            port_range: (40200..40300).into(),
            handler: Handler,
        });
        let client = id(50002);
        manager
            .get_password(&client, "user", PasswordAlgorithm::Md5)
            .await;
        manager
            .allocate(&client, None)
            .expect("allocation should succeed");

        let peer = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("peer socket should bind");
        let peer_address = peer.local_addr().expect("peer address should be available");
        assert!(manager.create_permission(&client, &[peer_address]));

        let relay = manager
            .relay_to_peer(&client, peer_address)
            .expect("permission should expose the relay socket");
        relay
            .send_to(b"request", peer_address)
            .await
            .expect("relay should send to arbitrary peer");

        let mut received = [0; 32];
        let (size, relay_address) = timeout(Duration::from_secs(1), peer.recv_from(&mut received))
            .await
            .expect("peer receive should not time out")
            .expect("peer receive should succeed");
        assert_eq!(&received[..size], b"request");
        peer.send_to(b"response", relay_address)
            .await
            .expect("peer should send response to relay");

        let (size, source) = timeout(Duration::from_secs(1), relay.recv_from(&mut received))
            .await
            .expect("relay receive should not time out")
            .expect("relay receive should succeed");
        assert_eq!(source, peer_address);
        assert_eq!(&received[..size], b"response");
    }

    #[tokio::test]
    async fn channel_binding_installs_permission_for_peer_socket() {
        let manager = SessionManager::new(SessionManagerOptions {
            port_range: (40100..40200).into(),
            handler: Handler,
        });
        let client = id(50001);
        manager
            .get_password(&client, "user", PasswordAlgorithm::Md5)
            .await;
        manager
            .allocate(&client, None)
            .expect("allocation should succeed");
        let peer = SocketAddr::from(([127, 0, 0, 1], 59998));

        assert!(manager.bind_channel(&client, peer, 0x4000));
        assert!(manager.relay_to_peer(&client, peer).is_some());
        assert_eq!(
            manager
                .relay_from_peer(&client, peer)
                .expect("peer should be permitted")
                .channel,
            Some(0x4000)
        );
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
