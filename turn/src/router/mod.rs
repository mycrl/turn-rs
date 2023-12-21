pub mod channels;
pub mod interfaces;
pub mod nodes;
pub mod nonces;
pub mod ports;

#[rustfmt::skip]
use crate::Observer;
use self::{
    channels::Channels,
    interfaces::{Interface, Interfaces},
    nodes::Nodes,
    nonces::Nonces,
    ports::Ports,
};

use std::{net::SocketAddr, sync::Arc, thread, time::Duration};

/// Router State Tree.
///
/// this state management example maintains the status of all
/// nodes in the current service and adds a node grouping model.
/// it is necessary to specify a group for each node.
///
/// The state between groups is isolated. However,
/// it should be noted that the node key only supports
/// long-term valid passwords，does not support short-term
/// valid passwords.
pub struct Router {
    realm: String,
    observer: Arc<dyn Observer>,
    ports: Ports,
    nonces: Nonces,
    nodes: Nodes,
    channels: Channels,
    interfaces: Interfaces,
}

impl Router {
    /// create a router.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    /// impl Observer for ObserverTest {}
    ///
    /// Router::new("test".to_string(), Arc::new(ObserverTest));
    /// ```
    pub fn new(realm: String, observer: Arc<dyn Observer>) -> Arc<Self> {
        let this = Arc::new(Self {
            interfaces: Interfaces::default(),
            channels: Channels::default(),
            nonces: Nonces::default(),
            ports: Ports::default(),
            nodes: Nodes::default(),
            observer,
            realm,
        });

        let this_ = Arc::downgrade(&this);
        thread::spawn(move || {
            while let Some(this) = this_.upgrade() {
                thread::sleep(Duration::from_secs(60));
                this.nodes.get_deaths().iter().for_each(|a| {
                    this.remove(a);
                });

                this.channels.get_deaths().iter().for_each(|c| {
                    this.channels.remove(*c);
                });
            }
        });

        this
    }

    /// get router capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    /// impl Observer for ObserverTest {}
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// assert_eq!(router.capacity(), 16383);
    /// ```
    pub fn capacity(&self) -> usize {
        self.ports.capacity()
    }

    /// get router allocate size.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    /// impl Observer for ObserverTest {}
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// assert_eq!(router.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.ports.len()
    }

    /// get router allocate size is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    /// impl Observer for ObserverTest {}
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// assert_eq!(router.is_empty(), true);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.ports.len() == 0
    }

    /// get addr interface.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    ///
    /// let interface = router.get_interface(&addr).unwrap();
    /// assert_eq!(interface.addr, addr);
    /// ```
    pub fn get_interface(&self, addr: &SocketAddr) -> Option<Arc<Interface>> {
        self.interfaces.get_ref(addr)
    }

    /// get user list.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    ///
    /// let users = router.get_users(0, 10);
    /// assert_eq!(users.as_slice(), &[("test".to_string(), vec![addr])]);
    /// ```
    pub fn get_users(&self, skip: usize, limit: usize) -> Vec<(String, Vec<SocketAddr>)> {
        self.nodes.get_users(skip, limit)
    }

    /// get node.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    ///
    /// let node = router.get_node(&addr).unwrap();
    /// assert_eq!(node.username.as_str(), "test");
    /// assert_eq!(node.password.as_str(), "test");
    /// assert_eq!(node.secret.as_slice(), &secret);
    /// assert_eq!(node.channels, vec![]);
    /// assert_eq!(node.ports, vec![]);
    /// ```
    pub fn get_node(&self, addr: &SocketAddr) -> Option<nodes::Node> {
        self.nodes.get_node(addr)
    }

    /// get node bound list.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    ///
    /// let ret = router.get_node_addrs("test");
    /// assert_eq!(ret, vec![addr]);
    /// ```
    pub fn get_node_addrs(&self, username: &str) -> Vec<SocketAddr> {
        self.nodes.get_addrs(username)
    }

    /// get the nonce of the node SocketAddr.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    ///
    /// let nonce = router.get_nonce(&addr);
    /// assert_eq!(nonce.len(), 16);
    /// ```
    pub fn get_nonce(&self, addr: &SocketAddr) -> Arc<String> {
        self.nonces.get(addr)
    }

    /// get the password of the node SocketAddr.
    ///
    /// require remote control service to distribute keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    /// ```
    pub fn get_key_block(
        &self,
        addr: &SocketAddr,
        interface: &SocketAddr,
        external: &SocketAddr,
        username: &str,
    ) -> Option<Arc<[u8; 16]>> {
        let key = self.nodes.get_secret(addr);
        if key.is_some() {
            return key;
        }

        let pwd = self.observer.get_password_blocking(addr, username)?;
        let key = self.nodes.insert(addr, &self.realm, username, &pwd)?;
        self.interfaces.insert(*addr, *interface, *external);
        Some(key)
    }

    /// get the password of the node SocketAddr.
    ///
    /// require remote control service to distribute keys.
    pub async fn get_key(
        &self,
        addr: &SocketAddr,
        interface: &SocketAddr,
        external: &SocketAddr,
        username: &str,
    ) -> Option<Arc<[u8; 16]>> {
        let key = self.nodes.get_secret(addr);
        if key.is_some() {
            return key;
        }

        let pwd = self.observer.get_password(addr, username).await?;
        let key = self.nodes.insert(addr, &self.realm, username, &pwd)?;
        self.interfaces.insert(*addr, *interface, *external);
        Some(key)
    }

    /// obtain the peer address bound to the current
    /// node according to the channel number.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    ///
    /// let port = router.alloc_port(&addr).unwrap();
    /// assert!(router.bind_port(&addr, port).is_some());
    /// assert_eq!(router.get_port_bound(port), Some(addr));
    /// ```
    pub fn get_channel_bound(&self, addr: &SocketAddr, channel: u16) -> Option<SocketAddr> {
        self.channels.get_bound(addr, channel)
    }

    /// obtain the peer address bound to the current
    /// node according to the port number.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    ///
    /// let port = router.alloc_port(&addr).unwrap();
    /// assert!(router.bind_port(&addr, port).is_some());
    /// assert_eq!(router.get_port_bound(port), Some(addr));
    /// ```
    pub fn get_port_bound(&self, port: u16) -> Option<SocketAddr> {
        self.ports.get(port)
    }

    /// get node the port.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    ///
    /// let port = router.alloc_port(&addr).unwrap();
    /// assert!(router.bind_port(&addr, port).is_some());
    /// assert!(router.bind_port(&peer, port).is_some());
    /// assert_eq!(router.get_bound_port(&addr, &peer), Some(port));
    /// ```
    pub fn get_bound_port(&self, addr: &SocketAddr, peer: &SocketAddr) -> Option<u16> {
        self.ports.get_bound(addr, peer)
    }

    /// alloc a port from State.
    ///
    /// In all cases, the server SHOULD only allocate ports from the range
    /// 49152 - 65535 (the Dynamic and/or Private Port range [PORT-NUMBERS]),
    /// unless the TURN server application knows, through some means not
    /// specified here, that other applications running on the same host as
    /// the TURN server application will not be impacted by allocating ports
    /// outside this range.  This condition can often be satisfied by running
    /// the TURN server application on a dedicated machine and/or by
    /// arranging that any other applications on the machine allocate ports
    /// before the TURN server application starts.  In any case, the TURN
    /// server SHOULD NOT allocate ports in the range 0 - 1023 (the Well-
    /// Known Port range) to discourage clients from using TURN to run
    /// standard services.
    ///
    ///   NOTE: The use of randomized port assignments to avoid certain
    ///   types of attacks is described in [RFC6056].  It is RECOMMENDED
    ///   that a TURN server implement a randomized port assignment
    ///   algorithm from [RFC6056].  This is especially applicable to
    ///   servers that choose to pre-allocate a number of ports from the
    ///   underlying OS and then later assign them to allocations; for
    ///   example, a server may choose this technique to implement the
    ///   EVEN-PORT attribute.
    ///
    /// The server determines the initial value of the time-to-expiry field
    /// as follows.  If the request contains a LIFETIME attribute, then the
    /// server computes the minimum of the client's proposed lifetime and the
    /// server's maximum allowed lifetime.  If this computed value is greater
    /// than the default lifetime, then the server uses the computed lifetime
    /// as the initial value of the time-to-expiry field.  Otherwise, the
    /// server uses the default lifetime.  It is RECOMMENDED that the server
    /// use a maximum allowed lifetime value of no more than 3600 seconds (1
    /// hour).  Servers that implement allocation quotas or charge users for
    /// allocations in some way may wish to use a smaller maximum allowed
    /// lifetime (perhaps as small as the default lifetime) to more quickly
    /// remove orphaned allocations (that is, allocations where the
    /// corresponding client has crashed or terminated, or the client
    /// connection has been lost for some reason).  Also, note that the time-
    /// to-expiry is recomputed with each successful Refresh request, and
    /// thus, the value computed here applies only until the first refresh.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    /// assert!(router.alloc_port(&addr).is_some());
    /// ```
    pub fn alloc_port(&self, addr: &SocketAddr) -> Option<u16> {
        let port = self.ports.alloc(addr)?;
        self.nodes.push_port(addr, port);
        Some(port)
    }

    /// bind port for State.
    ///
    /// A server need not do anything special to implement
    /// idempotency of CreatePermission requests over UDP using the
    /// "stateless stack approach".  Retransmitted CreatePermission
    /// requests will simply refresh the permissions.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    ///
    /// let port = router.alloc_port(&addr).unwrap();
    /// assert!(router.bind_port(&addr, port).is_some());
    /// ```
    pub fn bind_port(&self, addr: &SocketAddr, port: u16) -> Option<()> {
        self.ports.bound(addr, port)
    }

    /// bind channel number for State.
    ///
    /// A server need not do anything special to implement
    /// idempotency of ChannelBind requests over UDP using the
    /// "stateless stack approach".  Retransmitted ChannelBind requests
    /// will simply refresh the channel binding and the corresponding
    /// permission.  Furthermore, the client must wait 5 minutes before
    /// binding a previously bound channel number or peer address to a
    /// different channel, eliminating the possibility that the
    /// transaction would initially fail but succeed on a
    /// retransmission.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    ///
    /// let port = router.alloc_port(&addr).unwrap();
    /// assert!(router.bind_channel(&addr, port, 0x4000).is_some());
    /// ```
    pub fn bind_channel(&self, addr: &SocketAddr, port: u16, channel: u16) -> Option<()> {
        let source = self.ports.get(port)?;
        self.channels.insert(addr, channel, &source)?;
        self.nodes.push_channel(addr, channel)?;
        Some(())
    }

    /// refresh node lifetime.
    ///
    /// The server computes a value called the "desired lifetime" as follows:
    /// if the request contains a LIFETIME attribute and the attribute value
    /// is zero, then the "desired lifetime" is zero.  Otherwise, if the
    /// request contains a LIFETIME attribute, then the server computes the
    /// minimum of the client's requested lifetime and the server's maximum
    /// allowed lifetime.  If this computed value is greater than the default
    /// lifetime, then the "desired lifetime" is the computed value.
    /// Otherwise, the "desired lifetime" is the default lifetime.
    ///
    /// Subsequent processing depends on the "desired lifetime" value:
    ///
    /// * If the "desired lifetime" is zero, then the request succeeds and the
    ///   allocation is deleted.
    ///
    /// * If the "desired lifetime" is non-zero, then the request succeeds and
    ///   the allocation's time-to-expiry is set to the "desired lifetime".
    ///
    /// If the request succeeds, then the server sends a success response
    /// containing:
    ///
    /// * A LIFETIME attribute containing the current value of the time-to-
    ///   expiry timer.
    ///
    /// NOTE: A server need not do anything special to implement
    /// idempotency of Refresh requests over UDP using the "stateless
    /// stack approach".  Retransmitted Refresh requests with a non-
    /// zero "desired lifetime" will simply refresh the allocation.  A
    /// retransmitted Refresh request with a zero "desired lifetime"
    /// will cause a 437 (Allocation Mismatch) response if the
    /// allocation has already been deleted, but the client will treat
    /// this as equivalent to a success response (see below).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    /// router.refresh(&addr, 0);
    ///
    /// assert!(router.get_node(&addr).is_none());
    /// ```
    pub fn refresh(&self, addr: &SocketAddr, delay: u32) {
        if delay > 0 {
            self.nodes.set_lifetime(addr, delay);
        } else {
            self.remove(addr);
        }
    }

    /// remove a node.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    /// assert!(router.remove(&addr).is_some());
    /// assert!(router.get_node(&addr).is_none());
    /// ```
    pub fn remove(&self, addr: &SocketAddr) -> Option<()> {
        let node = self.nodes.remove(addr)?;
        self.ports.remove(addr, &node.ports);
        for c in node.channels {
            self.channels.remove(c);
        }

        self.nonces.remove(addr);
        self.interfaces.remove(addr);
        self.observer.abort(addr, &node.username);
        Some(())
    }

    /// remove a node from username.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use turn_rs::router::*;
    /// use turn_rs::*;
    ///
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {
    ///     fn get_password_blocking(&self, _: &SocketAddr, _: &str) -> Option<String> {
    ///         Some("test".to_string())
    ///     }
    /// }
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let secret = [
    ///     174, 238, 187, 253, 117, 209, 73, 157, 36, 56, 143, 91, 155, 16, 224, 239,
    /// ];
    ///
    /// let router = Router::new("test".to_string(), Arc::new(ObserverTest));
    /// let key = router.get_key_block(&addr, &addr, &addr, "test").unwrap();
    ///
    /// assert_eq!(key.as_slice(), &secret);
    /// router.remove_from_user("test");
    ///
    /// assert!(router.get_node(&addr).is_none());
    /// ```
    pub fn remove_from_user(&self, u: &str) {
        for addr in self.nodes.get_addrs(u) {
            self.remove(&addr);
        }
    }
}
