mod bucket_table;
mod nonce_table;
mod channel;
mod node;

use node::Node;
use channel::Channel;
use nonce_table::NonceTable;
use bucket_table::BucketTable;
use stun::util::long_key;
use tokio::sync::RwLock;
use tokio::time::{
    Duration,
    sleep
};

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc
};

use super::{
    config::Configure,
    broker::Broker
};

type Addr = Arc<SocketAddr>;

pub struct State {
    conf: Arc<Configure>,
    broker: Arc<Broker>,
    nonces: NonceTable,
    buckets: BucketTable,
    nodes: RwLock<HashMap<Addr, Node>>,
    ports: RwLock<HashMap<(u32, u16), Addr>>,
    port_bonds: RwLock<HashMap<Addr, HashMap<Addr, u16>>>,
    channels: RwLock<HashMap<(u32, u16), Channel>>,
    channel_bonds: RwLock<HashMap<(Addr, u16), Addr>>,
}

impl State {
    pub fn new(c: &Arc<Configure>, b: &Arc<Broker>) -> Arc<Self> {
        Arc::new(Self {
            conf: c.clone(),
            broker: b.clone(),
            nonces: NonceTable::new(),
            buckets: BucketTable::new(),
            nodes: RwLock::new(HashMap::with_capacity(1024)),
            ports: RwLock::new(HashMap::with_capacity(1024)),
            port_bonds: RwLock::new(HashMap::with_capacity(1024)),
            channels: RwLock::new(HashMap::with_capacity(1024)),
            channel_bonds: RwLock::new(HashMap::with_capacity(1024))
        })
    }

    pub async fn get_nonce(&self, a: &Addr) -> Arc<String> {
        self.nonces.get(a).await
    }

    pub async fn get_key(&self, a: &Addr, u: &str) -> Option<Arc<[u8; 16]>> {
        let key = self.nodes
            .read()
            .await
            .get(a)
            .map(|n| n.get_password());
        if key.is_some() {
            return key
        }

        let auth = match self.broker.auth(a, u).await {
            Ok(a) => a,
            Err(_) => return None
        };
        
        let node = Node::new(
            auth.group, 
            long_key(
                u, 
                &auth.password, 
                &self.conf.realm
            )
        );

        let key = node.get_password();
        self.nodes
            .write()
            .await
            .insert(a.clone(), node);
        Some(key)
    }

    pub async fn get_channel_bond(&self, a: &Addr, c: u16) -> Option<Addr> {
        self.channel_bonds
            .read()
            .await
            .get(&(a.clone(), c))
            .cloned()
    }

    pub async fn get_port_bond(&self, a: &Addr, p: u16) -> Option<Addr> {
        let g = self.nodes
            .read()
            .await
            .get(a)?
            .group;
        self.ports
            .read()
            .await
            .get(&(g, p))
            .cloned()
    }

    pub async fn get_node_port(&self, a: &Addr, p: &Addr) -> Option<u16> {
        self.port_bonds
            .read()
            .await
            .get(p)?
            .get(a)
            .copied()
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
    #[rustfmt::skip]
    pub async fn alloc_port(&self, a: &Addr) -> Option<u16> {
        let mut nodes = self.nodes.write().await;
        let node = nodes.get_mut(a)?;
        let port = self.buckets
            .alloc(node.group)
            .await?;
        self.ports
            .write()
            .await
            .insert((node.group, port), a.clone());
        if !node.ports.contains(&port) {
            node.ports.push(port);    
        }
        
        Some(port)
    }
    
    /// bind port for State.
    ///
    /// A server need not do anything special to implement
    /// idempotency of CreatePermission requests over UDP using the
    /// "stateless stack approach".  Retransmitted CreatePermission
    /// requests will simply refresh the permissions.
    #[rustfmt::skip]
    pub async fn bind_port(&self, a: &Addr, port: u16) -> Option<()> {
        let g = self.nodes
            .read()
            .await
            .get(a)?
            .group;
        let p = self.ports
            .read()
            .await
            .get(&(g, port))?
            .clone();
        self.port_bonds
            .write()
            .await
            .entry(a.clone())
            .or_insert_with(|| HashMap::with_capacity(10))
            .entry(p)
            .or_insert(port);
        Some(())
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
    #[rustfmt::skip]
    pub async fn bind_channel(&self, a: &Addr, p: u16, c: u16) -> Option<()> {
        let ports = self.ports.read().await;
        let mut channels = self.channels.write().await;
        let mut nodes = self.nodes.write().await;
        let mut is_empty = false;

        let node = nodes.get_mut(a)?;
        let channel = channels
            .entry((node.group, c))
            .or_insert_with(|| {
                is_empty = true;
                Channel::new(a)    
            });

        let is_include = if !is_empty {
            channel.includes(a)
        } else {
            true 
        };
        
        if !channel.is_half() && !is_include {
            return None
        }
        
        if !is_include {
            channel.up(a);
        }

        if !is_empty && is_include {
            channel.refresh();
        }

        let source = ports.get(&(node.group, p))?;
        if !node.channels.contains(&c) {
            node.channels.push(c)
        }

        self.channel_bonds
            .write()
            .await
            .entry((a.clone(), c))
            .or_insert(source.clone());
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
    /// *  If the "desired lifetime" is zero, then the request succeeds and
    ///    the allocation is deleted.
    /// 
    /// *  If the "desired lifetime" is non-zero, then the request succeeds
    ///    and the allocation's time-to-expiry is set to the "desired
    ///    lifetime".
    /// 
    /// If the request succeeds, then the server sends a success response
    /// containing:
    /// 
    /// *  A LIFETIME attribute containing the current value of the time-to-
    ///    expiry timer.
    /// 
    /// NOTE: A server need not do anything special to implement
    /// idempotency of Refresh requests over UDP using the "stateless
    /// stack approach".  Retransmitted Refresh requests with a non-
    /// zero "desired lifetime" will simply refresh the allocation.  A
    /// retransmitted Refresh request with a zero "desired lifetime"
    /// will cause a 437 (Allocation Mismatch) response if the
    /// allocation has already been deleted, but the client will treat
    /// this as equivalent to a success response (see below).
    #[rustfmt::skip]
    pub async fn refresh(&self, a: &Addr, delay: u32) {
        if delay == 0 { 
            self.remove(a).await; 
        } else {
            self.nodes
                .write()
                .await
                .get_mut(a)
                .map(|n| n.set_lifetime(delay));
        }
    }

    /// remove a node.
    #[rustfmt::skip]
    pub async fn remove(&self, a: &Addr) {
        let mut ports = self.ports.write().await;

        let node = match self.nodes.write().await.remove(a) {
            Some(n) => n,
            None => return
        };

        for p in node.ports {
            self.buckets.remove(node.group, p).await;
            ports.remove(&(node.group, p));
        }

        for c in node.channels {
            self.remove_channel(node.group, c).await;
        }

        self.nonces.remove(a).await;
        self.port_bonds
            .write()
            .await
            .remove(a);
    }
    
    /// remove channel in State. 
    #[rustfmt::skip]
    pub async fn remove_channel(&self, g: u32, c: u16) -> Option<()> {
        let mut channel_bonds = self.channel_bonds
            .write()
            .await;
        let mut channels = self.channels
            .write()
            .await;
        let channel = channels
            .remove(&(g, c))?;
        for a in channel {
            channel_bonds.remove(&(a, c));
        }
        
        Some(())
    }
    
    /// poll in State.
    #[rustfmt::skip]
    pub async fn poll(&self) {
        let fail_nodes = self.nodes
            .read()
            .await
            .iter()
            .filter(|(_, v)| v.is_death())
            .map(|(k, _)| k.clone())
            .collect::<Vec<Addr>>();
        for a in &fail_nodes {
            self.remove(a).await;
        }
        
        let fail_channels = self.channels
            .read()
            .await
            .iter()
            .filter(|(_, v)| v.is_death())
            .map(|(k, _)| *k)
            .collect::<Vec<(u32, u16)>>();
        for (g, c) in fail_channels {
            self.remove_channel(g, c).await;
        }
    }

    #[rustfmt::skip]
    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        let delay = Duration::from_secs(60);
        tokio::spawn(async move { 
            loop {
                sleep(delay).await;
                self.poll().await;
            }
        }).await?;
        Ok(())
    }
}

