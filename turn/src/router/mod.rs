pub mod nodes;
mod channels;
mod nonces;
mod ports;

use nodes::Nodes;
use ports::Ports;
use nonces::Nonces;
use channels::Channels;
use stun::util::long_key;
use tokio::time::{
    Duration,
    sleep,
};

use std::{
    net::SocketAddr,
    sync::Arc,
};

use crate::{
    Options,
    Observer,
};

type Addr = Arc<SocketAddr>;

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
    opt: Arc<Options>,
    observer: Arc<dyn Observer>,
    ports: Ports,
    nonces: Nonces,
    nodes: Nodes,
    channels: Channels,
}

impl Router {
    /// create a router.
    ///
    /// ```ignore
    /// struct Events;
    ///
    /// impl Observer for Events {
    ///     async fn auth(&self, _addr: &SocketAddr, _name: &str) -> Option<&str> {
    ///         Some("test")
    ///     }
    /// }
    ///
    /// let _router = Router::new(
    ///     Arc::new(Options {
    ///         external: "127.0.0.1:4378".parse().unwrap(),
    ///         realm: "test".to_string(),
    ///     }),
    ///     Arc::new(Events {}),
    /// );
    /// ```
    pub(crate) fn new(
        opt: Arc<Options>,
        observer: Arc<dyn Observer>,
    ) -> Arc<Self> {
        Arc::new(Self {
            channels: Channels::new(),
            nonces: Nonces::new(),
            ports: Ports::new(),
            nodes: Nodes::new(),
            observer,
            opt,
        })
    }

    /// poll in state.
    ///
    /// ```ignore
    /// let state = Router::new(/* ... */);
    ///
    /// tokio::spawn(state.start_poll());
    /// ```
    pub async fn start_poll(&self) {
        let delay = Duration::from_secs(60);
        loop {
            sleep(delay).await;
            for a in self.nodes.get_deaths().await {
                self.remove(&a).await;
            }

            for c in self.channels.get_deaths().await {
                self.channels.remove(c).await;
            }
        }
    }

    /// get router capacity.
    pub async fn capacity(&self) -> usize {
        self.ports.capacity().await
    }

    /// get router allocate size.
    pub async fn len(&self) -> usize {
        self.ports.len().await
    }

    /// get user list.
    ///
    /// ```ignore
    /// let router = Router::new(/* ... */);
    ///
    /// assert!(router.get_users().len() == 0);
    /// ```
    pub async fn get_users(&self) -> Vec<(String, Vec<SocketAddr>)> {
        self.nodes.get_users().await
    }

    /// get node.
    ///
    /// ```ignore
    /// let router = Router::new(/* ... */);
    ///
    /// assert!(router.get_node().is_none());
    /// ```
    pub async fn get_node(&self, a: &Addr) -> Option<nodes::Node> {
        self.nodes.get_node(a).await
    }

    /// get node bond list.
    ///
    /// ```ignore
    /// let router = Router::new(/* ... */);
    ///
    /// assert!(router.get_node_bonds().len() == 0);
    /// ```
    pub async fn get_node_bonds(&self, u: &str) -> Vec<Addr> {
        self.nodes.get_bond(u).await
    }

    /// get the nonce of the node SocketAddr.
    ///
    /// ```ignore
    /// let state = Router::new(/* ... */);
    ///
    /// assert!(state.get_nonce(&addr).len() == 16);
    /// ```
    pub async fn get_nonce(&self, a: &Addr) -> Arc<String> {
        self.nonces.get(a).await
    }

    /// get the password of the node SocketAddr.
    ///
    /// require remote control service to distribute keys.
    ///
    /// ```ignore
    /// let state = Router::new(/* ... */);
    ///
    /// // state.get_key(&addr, "panda")
    /// ```
    pub async fn get_key(&self, a: &Addr, u: &str) -> Option<Arc<[u8; 16]>> {
        let key = self.nodes.get_secret(a).await;
        if key.is_some() {
            return key;
        }

        let pwd = self.observer.auth(a.as_ref(), u).await?;
        let key = long_key(u, &pwd, &self.opt.realm);
        self.nodes.insert(a, u, key, pwd).await
    }

    /// obtain the peer address bound to the current
    /// node according to the channel number.
    ///
    /// ```ignore
    /// let state = Router::new(/* ... */);
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    ///
    /// state.get_key(&addr, "panda");
    /// state.get_key(&peer, "panda");
    ///
    /// let addr_port = state.alloc_port(&addr).unwrap();
    /// let peer_port = state.alloc_port(&peer).unwrap();
    ///
    /// state.bind_channel(&addr, peer_port, 0x4000);
    /// state.bind_channel(&peer, addr_port, 0x4000);
    ///
    /// assert_eq!(state.get_channel_bond(&addr, 0x4000).unwrap(), peer);
    /// ```
    pub async fn get_channel_bond(&self, a: &Addr, c: u16) -> Option<Addr> {
        self.channels.get_bond(a, c).await
    }

    /// obtain the peer address bound to the current
    /// node according to the port number.
    ///
    /// ```ignore
    /// let state = Router::new(/* ... */);
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    ///
    /// state.get_key(&addr, "panda");
    /// state.get_key(&peer, "panda");
    ///
    /// let addr_port = state.alloc_port(&addr).unwrap();
    /// let peer_port = state.alloc_port(&peer).unwrap();
    ///
    /// state.bind_port(&peer, addr_port);
    /// state.bind_port(&addr, peer_port);
    ///
    /// assert_eq!(state.get_port_bond(&addr, peer_port), some(peer));
    /// assert_eq!(state.get_port_bond(&peer, addr_port), some(addr));
    /// ```
    pub async fn get_port_bond(&self, p: u16) -> Option<Addr> {
        self.ports.get(p).await
    }

    /// get node the port.
    ///
    /// ```ignore
    /// let state = Router::new(/* ... */);
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    ///
    /// state.get_key(&addr, "panda");
    /// state.get_key(&peer, "panda");
    ///
    /// let addr_port = state.alloc_port(&addr).unwrap();
    /// let peer_port = state.alloc_port(&peer).unwrap();
    ///
    /// state.bind_port(&peer, addr_port);
    /// state.bind_port(&addr, peer_port);
    ///
    /// assert_eq!(state.get_bond_port(&addr, &peer), some(peer_port));
    /// assert_eq!(state.get_bond_port(&peer, &addr), some(addr_port));
    /// ```
    pub async fn get_bond_port(&self, a: &Addr, p: &Addr) -> Option<u16> {
        self.ports.get_bound(a, p).await
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
    /// ```ignore
    /// let state = Router::new(/* ... */);
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    ///
    /// state.get_key(&addr, "panda");
    /// state.get_key(&peer, "panda");
    ///
    /// assert!(state.alloc_port(&addr).unwrap().is_some());
    /// assert!(state.alloc_port(&peer).unwrap().is_some());
    /// ```
    pub async fn alloc_port(&self, a: &Addr) -> Option<u16> {
        let port = self.ports.alloc(a).await?;
        self.nodes.push_port(a, port).await;
        Some(port)
    }

    /// bind port for State.
    ///
    /// A server need not do anything special to implement
    /// idempotency of CreatePermission requests over UDP using the
    /// "stateless stack approach".  Retransmitted CreatePermission
    /// requests will simply refresh the permissions.
    ///
    /// ```ignore
    /// let state = Router::new(/* ... */);
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    ///
    /// state.get_key(&addr, "panda");
    /// state.get_key(&peer, "panda");
    ///
    /// let addr_port = state.alloc_port(&addr).unwrap();
    /// let peer_port = state.alloc_port(&peer).unwrap();
    ///
    /// assert!(state.bind_port(&peer, addr_port).is_some());
    /// assert!(state.bind_port(&addr, peer_port).is_some());
    /// ```
    pub async fn bind_port(&self, a: &Addr, port: u16) -> Option<()> {
        self.ports.bound(a, port).await
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
    /// ```ignore
    /// let state = Router::new(/* ... */);
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let peer = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
    ///
    /// state.get_key(&addr, "panda");
    /// state.get_key(&peer, "panda");
    ///
    /// let addr_port = state.alloc_port(&addr).unwrap();
    /// let peer_port = state.alloc_port(&peer).unwrap();
    ///
    /// assert!(state.bind_channel(&peer, addr_port, 0x4000).is_some());
    /// assert!(state.bind_channel(&addr, peer_port, 0x4000).is_some());
    /// ```
    pub async fn bind_channel(&self, a: &Addr, p: u16, c: u16) -> Option<()> {
        let source = self.ports.get(p).await?;
        self.channels.insert(a, c, &source).await?;
        self.nodes.push_channel(a, c).await?;
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
    /// ```ignore
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let state = Router::new(/* ... */);
    ///
    /// state.get_key(&addr, "panda");
    /// state.refresh(&addr, 600);
    /// state.refresh(&addr, 0);
    /// ```
    pub async fn refresh(&self, a: &Addr, delay: u32) {
        if delay > 0 {
            self.nodes.set_lifetime(a, delay).await;
        } else {
            self.remove(a).await;
        }
    }

    /// remove a node.
    ///
    /// ```ignore
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let state = Router::new(/* ... */);
    ///
    /// state.get_key(&addr, "panda");
    /// state.remove(&addr);
    /// ```
    pub async fn remove(&self, a: &Addr) -> Option<()> {
        let node = self.nodes.remove(a).await?;
        self.ports.remove(a, &node.ports).await;

        for c in node.channels {
            self.channels.remove(c).await?;
        }

        self.nonces.remove(a).await;
        Some(())
    }

    /// remove a node from username.
    ///
    /// ```ignore
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let state = Router::new(/* ... */);
    ///
    /// state.get_key(&addr, "panda");
    /// state.remove_from_user("panda");
    /// ```
    pub async fn remove_from_user(&self, u: &str) {
        for addr in self.nodes.get_bond(u).await {
            self.remove(&addr).await;
        }
    }
}
