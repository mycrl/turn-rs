use super::Observer;
use crate::stun::util::long_term_credential_digest;

use std::{
    hash::Hash,
    net::SocketAddr,
    ops::{Deref, DerefMut, Range},
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

/// Authentication information for the session.
///
/// Digest data is data that summarises usernames and passwords by means of
/// long-term authentication.
#[derive(Debug, Clone)]
pub struct Auth {
    pub username: String,
    pub integrity: [u8; 16],
}

/// Assignment information for the session.
///
/// Sessions are all bound to only one port and one channel.
#[derive(Debug, Clone)]
pub struct Allocate {
    pub port: Option<u16>,
    pub channels: Vec<u16>,
}

/// turn session information.
///
/// A user can have many sessions.
///
/// The default survival time for a session is 600 seconds.
#[derive(Debug, Clone)]
pub struct Session {
    pub auth: Auth,
    pub allocate: Allocate,
    pub permissions: Vec<u16>,
    pub expires: u64,
}

/// The identifier of the session or addr.
///
/// Each session needs to be identified by a combination of three pieces of
/// information: the addr address, and the transport protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionAddr {
    pub address: SocketAddr,
    pub interface: SocketAddr,
}

/// The addr used to record the current session.
///
/// This is used when forwarding data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Endpoint {
    pub address: SocketAddr,
    pub endpoint: SocketAddr,
}

/// A specially optimised timer.
///
/// This timer does not stack automatically and needs to be stacked externally
/// and manually.
///
/// ```
/// use turn_server::turn::sessions::Timer;
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

#[derive(Default)]
pub struct State {
    sessions: RwLock<Table<SessionAddr, Session>>,
    port_allocate_pool: Mutex<PortAllocatePools>,
    // Records the sessions corresponding to each assigned port, which will be needed when looking
    // up sessions assigned to this port based on the port number.
    port_mapping_table: RwLock<Table</* port */ u16, SessionAddr>>,
    // Records the nonce value for each network connection, which is independent of the session
    // because it can exist before it is authenticated.
    address_nonce_tanle: RwLock<Table<SessionAddr, (String, /* expires */ u64)>>,
    // Stores the address to which the session should be forwarded when it sends indication to a
    // port. This is written when permissions are created to allow a certain address to be
    // forwarded to the current session.
    port_relay_table: RwLock<Table<SessionAddr, HashMap</* port */ u16, Endpoint>>>,
    // Indicates to which session the data sent by a session to a channel should be forwarded.
    channel_relay_table: RwLock<Table<SessionAddr, HashMap</* channel */ u16, Endpoint>>>,
}

pub struct Sessions<T> {
    timer: Timer,
    state: State,
    observer: T,
}

impl<T: Observer + 'static> Sessions<T> {
    pub fn new(observer: T) -> Arc<Self> {
        let this = Arc::new(Self {
            state: State::default(),
            timer: Timer::default(),
            observer,
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
                        this.state
                            .sessions
                            .read()
                            .iter()
                            .filter(|(_, v)| v.expires <= now)
                            .for_each(|(k, _)| address.push(*k));
                    }

                    // Delete the expired sessions.
                    if !address.is_empty() {
                        this.remove_session(&address);
                        address.clear();
                    }
                }

                // Because nonce does not follow session creation, nonce is created for each
                // addr, so nonce deletion is handled independently.
                {
                    this.state
                        .address_nonce_tanle
                        .read()
                        .iter()
                        .filter(|(_, v)| v.1 <= now)
                        .for_each(|(k, _)| address.push(*k));

                    if !address.is_empty() {
                        this.remove_nonce(&address);
                        address.clear();
                    }
                }

                // Fixing a second tick.
                sleep(Duration::from_secs(1));
            }
        });

        this
    }

    fn remove_session(&self, addrs: &[SessionAddr]) {
        let mut sessions = self.state.sessions.write();
        let mut port_allocate_pool = self.state.port_allocate_pool.lock();
        let mut port_mapping_table = self.state.port_mapping_table.write();
        let mut port_relay_table = self.state.port_relay_table.write();
        let mut channel_relay_table = self.state.channel_relay_table.write();

        addrs.iter().for_each(|k| {
            port_relay_table.remove(k);
            channel_relay_table.remove(k);

            if let Some(session) = sessions.remove(k) {
                // Removes the session-bound port from the port binding table and
                // releases the port back into the allocation pool.
                if let Some(port) = session.allocate.port {
                    port_mapping_table.remove(&port);
                    port_allocate_pool.restore(port);
                }

                // Notifies that the external session has been closed.
                self.observer.closed(k, &session.auth.username);
            }
        });
    }

    fn remove_nonce(&self, addrs: &[SessionAddr]) {
        let mut address_nonce_tanle = self.state.address_nonce_tanle.write();

        addrs.iter().for_each(|k| {
            address_nonce_tanle.remove(k);
        });
    }

    /// Get session for addr.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password(&self, username: &str) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// assert!(sessions.get_session(&addr).get_ref().is_none());
    ///
    /// sessions.get_integrity(&addr, "test", "test");
    ///
    /// let lock = sessions.get_session(&addr);
    /// let session = lock.get_ref().unwrap();
    /// assert_eq!(session.auth.username, "test");
    /// assert_eq!(session.allocate.port, None);
    /// assert_eq!(session.allocate.channels.len(), 0);
    /// ```
    pub fn get_session<'a, 'b>(
        &'a self,
        key: &'b SessionAddr,
    ) -> ReadLock<'b, 'a, SessionAddr, Table<SessionAddr, Session>> {
        ReadLock {
            lock: self.state.sessions.read(),
            key,
        }
    }

    /// Get nonce for addr.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {}
    ///
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// let a = sessions.get_nonce(&addr).get_ref().unwrap().clone();
    /// assert!(a.0.len() == 16);
    /// assert!(a.1 == 600 || a.1 == 601 || a.1 == 602);
    ///
    /// let b = sessions.get_nonce(&addr).get_ref().unwrap().clone();
    /// assert_eq!(a.0, b.0);
    /// assert!(b.1 == 600 || b.1 == 601 || b.1 == 602);
    /// ```
    pub fn get_nonce<'a, 'b>(
        &'a self,
        key: &'b SessionAddr,
    ) -> ReadLock<'b, 'a, SessionAddr, Table<SessionAddr, (String, u64)>> {
        // If no nonce is created, create a new one.
        {
            if !self.state.address_nonce_tanle.read().contains_key(key) {
                self.state.address_nonce_tanle.write().insert(
                    *key,
                    (
                        // A random string of length 16.
                        {
                            let mut rng = rand::rng();
                            std::iter::repeat(())
                                .map(|_| rng.sample(Alphanumeric) as char)
                                .take(16)
                                .collect::<String>()
                                .to_lowercase()
                        },
                        // Current time stacks for 600 seconds.
                        self.timer.get() + 600,
                    ),
                );
            }
        }

        ReadLock {
            lock: self.state.address_nonce_tanle.read(),
            key,
        }
    }

    /// Get digest for addr.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password(&self, username: &str) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// assert_eq!(sessions.get_integrity(&addr, "test1", "test"), None);
    ///
    /// assert_eq!(sessions.get_integrity(&addr, "test", "test"), Some(digest));
    ///
    /// assert_eq!(sessions.get_integrity(&addr, "test", "test"), Some(digest));
    /// ```
    pub fn get_integrity(&self, addr: &SessionAddr, username: &str, realm: &str) -> Option<[u8; 16]> {
        // Already authenticated, get the cached digest directly.
        {
            if let Some(it) = self.state.sessions.read().get(addr) {
                return Some(it.auth.integrity);
            }
        }

        // Get the current user's password from an external observer and create a
        // digest.
        let password = self.observer.get_password(username)?;
        let integrity = long_term_credential_digest(&username, &password, realm);

        // Record a new session.
        {
            self.state.sessions.write().insert(
                *addr,
                Session {
                    permissions: Vec::with_capacity(10),
                    expires: self.timer.get() + 600,
                    auth: Auth {
                        username: username.to_string(),
                        integrity,
                    },
                    allocate: Allocate {
                        channels: Vec::with_capacity(10),
                        port: None,
                    },
                },
            );
        }

        Some(integrity)
    }

    pub fn allocated(&self) -> usize {
        self.state.port_allocate_pool.lock().len()
    }

    /// Assign a port number to the session.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password(&self, username: &str) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// sessions.get_integrity(&addr, "test", "test");
    ///
    /// {
    ///     let lock = sessions.get_session(&addr);
    ///     let session = lock.get_ref().unwrap();
    ///     assert_eq!(session.auth.username, "test");
    ///     assert_eq!(session.allocate.port, None);
    ///     assert_eq!(session.allocate.channels.len(), 0);
    /// }
    ///
    /// let port = sessions.allocate(&addr).unwrap();
    /// {
    ///     let lock = sessions.get_session(&addr);
    ///     let session = lock.get_ref().unwrap();
    ///     assert_eq!(session.auth.username, "test");
    ///     assert_eq!(session.allocate.port, Some(port));
    ///     assert_eq!(session.allocate.channels.len(), 0);
    /// }
    ///
    /// assert_eq!(sessions.allocate(&addr), Some(port));
    /// ```
    pub fn allocate(&self, addr: &SessionAddr) -> Option<u16> {
        let mut lock = self.state.sessions.write();
        let session = lock.get_mut(addr)?;

        // If the port has already been allocated, re-allocation is not allowed.
        if let Some(port) = session.allocate.port {
            return Some(port);
        }

        // Records the port assigned to the current session and resets the alive time.
        let port = self.state.port_allocate_pool.lock().alloc(None)?;
        session.expires = self.timer.get() + 600;
        session.allocate.port = Some(port);

        // Write the allocation port binding table.
        self.state.port_mapping_table.write().insert(port, *addr);
        Some(port)
    }

    /// Create permission for session.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password(&self, username: &str) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let peer_addr = SessionAddr {
    ///     address: "127.0.0.1:8081".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// sessions.get_integrity(&addr, "test", "test");
    /// sessions.get_integrity(&peer_addr, "test", "test");
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
    pub fn create_permission(&self, addr: &SessionAddr, endpoint: &SocketAddr, ports: &[u16]) -> bool {
        let mut sessions = self.state.sessions.write();
        let mut port_relay_table = self.state.port_relay_table.write();
        let port_mapping_table = self.state.port_mapping_table.read();

        // Finds information about the current session.
        let session = if let Some(it) = sessions.get_mut(addr) {
            it
        } else {
            return false;
        };

        // The port number assigned to the current session.
        let local_port = if let Some(it) = session.allocate.port {
            it
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
                        address: addr.address,
                        endpoint: *endpoint,
                    },
                );

            // Do not store the same peer ports to the permission list over and over again.
            if !session.permissions.contains(&port) {
                session.permissions.push(port);
            }
        }

        true
    }

    /// Binding a channel to the session.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password(&self, username: &str) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let peer_addr = SessionAddr {
    ///     address: "127.0.0.1:8081".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// sessions.get_integrity(&addr, "test", "test");
    /// sessions.get_integrity(&peer_addr, "test", "test");
    ///
    /// let port = sessions.allocate(&addr).unwrap();
    /// let peer_port = sessions.allocate(&peer_addr).unwrap();
    /// assert_eq!(
    ///     sessions
    ///         .get_session(&addr)
    ///         .get_ref()
    ///         .unwrap()
    ///         .allocate
    ///         .channels
    ///         .len(),
    ///     0
    /// );
    ///
    /// assert_eq!(
    ///     sessions
    ///         .get_session(&peer_addr)
    ///         .get_ref()
    ///         .unwrap()
    ///         .allocate
    ///         .channels
    ///         .len(),
    ///     0
    /// );
    ///
    /// assert!(sessions.bind_channel(&addr, &endpoint, peer_port, 0x4000));
    /// assert!(sessions.bind_channel(&peer_addr, &endpoint, port, 0x4000));
    /// assert_eq!(
    ///     sessions
    ///         .get_session(&addr)
    ///         .get_ref()
    ///         .unwrap()
    ///         .allocate
    ///         .channels,
    ///     vec![0x4000]
    /// );
    ///
    /// assert_eq!(
    ///     sessions
    ///         .get_session(&peer_addr)
    ///         .get_ref()
    ///         .unwrap()
    ///         .allocate
    ///         .channels,
    ///     vec![0x4000]
    /// );
    /// ```
    pub fn bind_channel(&self, addr: &SessionAddr, endpoint: &SocketAddr, port: u16, channel: u16) -> bool {
        // Finds the address of the bound opposing port.
        let peer = if let Some(it) = self.state.port_mapping_table.read().get(&port) {
            *it
        } else {
            return false;
        };

        // Records the channel used for the current session.
        {
            let mut lock = self.state.sessions.write();
            let session = if let Some(it) = lock.get_mut(addr) {
                it
            } else {
                return false;
            };

            if !session.allocate.channels.contains(&channel) {
                session.allocate.channels.push(channel);
            }
        }

        // Binding ports also creates permissions.
        if !self.create_permission(addr, endpoint, &[port]) {
            return false;
        }

        // Create channel forwarding mapping relationships for peers.
        self.state
            .channel_relay_table
            .write()
            .entry(peer)
            .or_insert_with(|| HashMap::with_capacity(10))
            .insert(
                channel,
                Endpoint {
                    address: addr.address,
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
    /// use turn_server::turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password(&self, username: &str) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let peer_addr = SessionAddr {
    ///     address: "127.0.0.1:8081".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// sessions.get_integrity(&addr, "test", "test");
    /// sessions.get_integrity(&peer_addr, "test", "test");
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
    pub fn get_channel_relay_address(&self, addr: &SessionAddr, channel: u16) -> Option<Endpoint> {
        self.state.channel_relay_table.read().get(&addr)?.get(&channel).copied()
    }

    /// Get the address of the port binding.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password(&self, username: &str) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let endpoint = "127.0.0.1:3478".parse().unwrap();
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let peer_addr = SessionAddr {
    ///     address: "127.0.0.1:8081".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// sessions.get_integrity(&addr, "test", "test");
    /// sessions.get_integrity(&peer_addr, "test", "test");
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
    pub fn get_relay_address(&self, addr: &SessionAddr, port: u16) -> Option<Endpoint> {
        self.state.port_relay_table.read().get(&addr)?.get(&port).copied()
    }

    /// Refresh the session for addr.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password(&self, username: &str) -> Option<String> {
    ///         if username == "test" {
    ///             Some("test".to_string())
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    ///
    /// let addr = SessionAddr {
    ///     address: "127.0.0.1:8080".parse().unwrap(),
    ///     interface: "127.0.0.1:3478".parse().unwrap(),
    /// };
    ///
    /// let digest = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224,
    ///     239,
    /// ];
    ///
    /// let sessions = Sessions::new(ObserverTest);
    ///
    /// assert!(sessions.get_session(&addr).get_ref().is_none());
    ///
    /// sessions.get_integrity(&addr, "test", "test");
    ///
    /// let expires = sessions.get_session(&addr).get_ref().unwrap().expires;
    /// assert!(expires == 600 || expires == 601 || expires == 602);
    ///
    /// assert!(sessions.refresh(&addr, 0));
    ///
    /// assert!(sessions.get_session(&addr).get_ref().is_none());
    /// ```
    pub fn refresh(&self, addr: &SessionAddr, lifetime: u32) -> bool {
        if lifetime > 3600 {
            return false;
        }

        if lifetime == 0 {
            self.remove_session(&[*addr]);
            self.remove_nonce(&[*addr]);
        } else {
            if let Some(session) = self.state.sessions.write().get_mut(addr) {
                session.expires = self.timer.get() + lifetime as u64;
            } else {
                return false;
            }

            if let Some(nonce) = self.state.address_nonce_tanle.write().get_mut(addr) {
                nonce.1 = self.timer.get() + lifetime as u64;
            }
        }

        true
    }
}

/// The default HashMap is created without allocating capacity. To improve
/// performance, the turn server needs to pre-allocate the available capacity.
///
/// So here the HashMap is rewrapped to allocate a large capacity (number of
/// ports that can be allocated) at the default creation time as well.
pub struct Table<K, V>(HashMap<K, V>);

impl<K, V> Default for Table<K, V> {
    fn default() -> Self {
        Self(HashMap::with_capacity(PortAllocatePools::capacity()))
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
    key: &'a K,
    lock: RwLockReadGuard<'b, R>,
}

impl<'a, 'b, K, V> ReadLock<'a, 'b, K, Table<K, V>>
where
    K: Eq + Hash,
{
    pub fn get_ref(&self) -> Option<&V> {
        self.lock.get(self.key)
    }
}

/// Bit Flag
#[derive(PartialEq, Eq)]
pub enum Bit {
    Low,
    High,
}

/// Random Port
///
/// Recently, awareness has been raised about a number of "blind" attacks
/// (i.e., attacks that can be performed without the need to sniff the
/// packets that correspond to the transport protocol instance to be
/// attacked) that can be performed against the Transmission Control
/// Protocol (TCP) [RFC0793] and similar protocols.  The consequences of
/// these attacks range from throughput reduction to broken connections
/// or data corruption [RFC5927] [RFC4953] [Watson].
///
/// All these attacks rely on the attacker's ability to guess or know the
/// five-tuple (Protocol, Source Address, Source port, Destination
/// Address, Destination Port) that identifies the transport protocol
/// instance to be attacked.
///
/// Services are usually located at fixed, "well-known" ports [IANA] at
/// the host supplying the service (the server).  Client applications
/// connecting to any such service will contact the server by specifying
/// the server IP address and service port number.  The IP address and
/// port number of the client are normally left unspecified by the client
/// application and thus are chosen automatically by the client
/// networking stack.  Ports chosen automatically by the networking stack
/// are known as ephemeral ports [Stevens].
///
/// While the server IP address, the well-known port, and the client IP
/// address may be known by an attacker, the ephemeral port of the client
/// is usually unknown and must be guessed.
///
/// # Test
///
/// ```
/// use std::collections::HashSet;
/// use turn_server::turn::sessions::*;
///
/// let mut pool = PortAllocatePools::default();
/// let mut ports = HashSet::with_capacity(PortAllocatePools::capacity());
///
/// while let Some(port) = pool.alloc(None) {
///     ports.insert(port);
/// }
///
/// assert_eq!(PortAllocatePools::capacity() + 1, ports.len());
/// ```
pub struct PortAllocatePools {
    pub buckets: Vec<u64>,
    allocated: usize,
    bit_len: u32,
    peak: usize,
}

impl Default for PortAllocatePools {
    fn default() -> Self {
        Self {
            buckets: vec![0; Self::bucket_size()],
            peak: Self::bucket_size() - 1,
            bit_len: Self::bit_len(),
            allocated: 0,
        }
    }
}

impl PortAllocatePools {
    /// compute bucket size.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::sessions::*;
    ///
    /// assert_eq!(PortAllocatePools::bucket_size(), 256);
    /// ```
    pub fn bucket_size() -> usize {
        (Self::capacity() as f32 / 64.0).ceil() as usize
    }

    /// compute bucket last bit max offset.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::sessions::*;
    ///
    /// assert_eq!(PortAllocatePools::bit_len(), 63);
    /// ```
    pub fn bit_len() -> u32 {
        (Self::capacity() as f32 % 64.0).ceil() as u32
    }

    /// get pools capacity.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::sessions::Bit;
    /// use turn_server::turn::sessions::PortAllocatePools;
    ///
    /// assert_eq!(PortAllocatePools::capacity(), 65535 - 49152);
    /// ```
    pub const fn capacity() -> usize {
        65535 - 49152
    }

    /// get port range.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::sessions::*;
    ///
    /// assert_eq!(PortAllocatePools::port_range(), 49152..65535);
    /// ```
    pub const fn port_range() -> Range<u16> {
        49152..65535
    }

    /// get pools allocated size.
    ///
    /// ```
    /// use turn_server::turn::sessions::PortAllocatePools;
    ///
    /// let mut pools = PortAllocatePools::default();
    /// assert_eq!(pools.len(), 0);
    ///
    /// pools.alloc(None).unwrap();
    /// assert_eq!(pools.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.allocated
    }

    /// get pools allocated size is empty.
    ///
    /// ```
    /// use turn_server::turn::sessions::PortAllocatePools;
    ///
    /// let mut pools = PortAllocatePools::default();
    /// assert_eq!(pools.len(), 0);
    /// assert_eq!(pools.is_empty(), true);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.allocated == 0
    }

    /// random assign a port.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::sessions::PortAllocatePools;
    ///
    /// let mut pool = PortAllocatePools::default();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// assert!(pool.alloc(None).is_some());
    /// ```
    pub fn alloc(&mut self, start_index: Option<usize>) -> Option<u16> {
        let mut index = None;
        let mut start = start_index.unwrap_or_else(|| rand::rng().random_range(0..self.peak as u16) as usize);

        // When the partition lookup has gone through the entire partition list, the
        // lookup should be stopped, and the location where it should be stopped is
        // recorded here.
        let previous = if start == 0 { self.peak } else { start - 1 };

        loop {
            // Finds the first high position in the partition.
            if let Some(i) = {
                let bucket = self.buckets[start];
                if bucket < u64::MAX {
                    let offset = bucket.leading_ones();

                    // Check to see if the jump is beyond the partition list or the lookup exceeds
                    // the maximum length of the allocation table.
                    if start == self.peak && offset > self.bit_len {
                        None
                    } else {
                        Some(offset)
                    }
                } else {
                    None
                }
            } {
                index = Some(i as usize);
                break;
            }

            // As long as it doesn't find it, it continues to re-find it from the next
            // partition.
            if start == self.peak {
                start = 0;
            } else {
                start += 1;
            }

            // Already gone through all partitions, lookup failed.
            if start == previous {
                break;
            }
        }

        // Writes to the partition, marking the current location as already allocated.
        let index = index?;
        self.set_bit(start, index, Bit::High);
        self.allocated += 1;

        // The actual port number is calculated from the partition offset position.
        let num = (start * 64 + index) as u16;
        let port = Self::port_range().start + num;
        Some(port)
    }

    /// write bit flag in the bucket.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::sessions::Bit;
    /// use turn_server::turn::sessions::PortAllocatePools;
    ///
    /// let mut pool = PortAllocatePools::default();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// pool.set_bit(0, 0, Bit::High);
    /// pool.set_bit(0, 1, Bit::High);
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49154));
    /// assert_eq!(pool.alloc(Some(0)), Some(49155));
    /// ```
    pub fn set_bit(&mut self, bucket: usize, index: usize, bit: Bit) {
        let high_mask = 1 << (63 - index);
        let mask = match bit {
            Bit::Low => u64::MAX ^ high_mask,
            Bit::High => high_mask,
        };

        let value = self.buckets[bucket];
        self.buckets[bucket] = match bit {
            Bit::High => value | mask,
            Bit::Low => value & mask,
        };
    }

    /// restore port in the buckets.
    ///
    /// # Test
    ///
    /// ```
    /// use turn_server::turn::sessions::PortAllocatePools;
    ///
    /// let mut pool = PortAllocatePools::default();
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    ///
    /// pool.restore(49152);
    /// pool.restore(49153);
    ///
    /// assert_eq!(pool.alloc(Some(0)), Some(49152));
    /// assert_eq!(pool.alloc(Some(0)), Some(49153));
    /// ```
    pub fn restore(&mut self, port: u16) {
        assert!(Self::port_range().contains(&port));

        // Calculate the location in the partition from the port number.
        let offset = (port - Self::port_range().start) as usize;
        let bucket = offset / 64;
        let index = offset - (bucket * 64);

        // Gets the bit value in the port position in the partition, if it is low, no
        // processing is required.
        if {
            match (self.buckets[bucket] & (1 << (63 - index))) >> (63 - index) {
                0 => Bit::Low,
                1 => Bit::High,
                _ => panic!(),
            }
        } == Bit::Low
        {
            return;
        }

        self.set_bit(bucket, index, Bit::Low);
        self.allocated -= 1;
    }
}
