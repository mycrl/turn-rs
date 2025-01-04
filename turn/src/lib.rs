pub mod operations;
pub mod sessions;

use self::operations::ServiceContext;

pub use self::{
    operations::{Operationer, ResponseMethod},
    sessions::{PortAllocatePools, Session, SessionAddr, Sessions},
};

use std::{future::Future, net::SocketAddr, sync::Arc};

#[rustfmt::skip]
static SOFTWARE: &str = concat!(
    "turn-rs.",
    env!("CARGO_PKG_VERSION")
);

#[allow(unused)]
pub trait Observer: Send + Sync {
    fn get_password(
        &self,
        addr: &SessionAddr,
        username: &str,
    ) -> impl Future<Output = Option<String>> + Send {
        async { None }
    }

    /// allocate request
    ///
    /// [rfc8489](https://tools.ietf.org/html/rfc8489)
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
    fn allocated(&self, addr: &SessionAddr, username: &str, port: u16) {}

    /// channel binding request
    ///
    /// The server MAY impose restrictions on the IP address and port values
    /// allowed in the XOR-PEER-ADDRESS attribute; if a value is not allowed,
    /// the server rejects the request with a 403 (Forbidden) error.
    ///
    /// If the request is valid, but the server is unable to fulfill the
    /// request due to some capacity limit or similar, the server replies
    /// with a 508 (Insufficient Capacity) error.
    ///
    /// Otherwise, the server replies with a ChannelBind success response.
    /// There are no required attributes in a successful ChannelBind
    /// response.
    ///
    /// If the server can satisfy the request, then the server creates or
    /// refreshes the channel binding using the channel number in the
    /// CHANNEL-NUMBER attribute and the transport address in the XOR-PEER-
    /// ADDRESS attribute.  The server also installs or refreshes a
    /// permission for the IP address in the XOR-PEER-ADDRESS attribute as
    /// described in Section 9.
    ///
    /// NOTE: A server need not do anything special to implement
    /// idempotency of ChannelBind requests over UDP using the
    /// "stateless stack approach".  Retransmitted ChannelBind requests
    /// will simply refresh the channel binding and the corresponding
    /// permission.  Furthermore, the client must wait 5 minutes before
    /// binding a previously bound channel number or peer address to a
    /// different channel, eliminating the possibility that the
    /// transaction would initially fail but succeed on a
    /// retransmission.
    fn channel_bind(&self, addr: &SessionAddr, username: &str, channel: u16) {}

    /// create permission request
    ///
    /// [rfc8489](https://tools.ietf.org/html/rfc8489)
    ///
    /// When the server receives the CreatePermission request, it processes
    /// as per [Section 5](https://tools.ietf.org/html/rfc8656#section-5)
    /// plus the specific rules mentioned here.
    ///
    /// The message is checked for validity.  The CreatePermission request
    /// MUST contain at least one XOR-PEER-ADDRESS attribute and MAY contain
    /// multiple such attributes.  If no such attribute exists, or if any of
    /// these attributes are invalid, then a 400 (Bad Request) error is
    /// returned.  If the request is valid, but the server is unable to
    /// satisfy the request due to some capacity limit or similar, then a 508
    /// (Insufficient Capacity) error is returned.
    ///
    /// If an XOR-PEER-ADDRESS attribute contains an address of an address
    /// family that is not the same as that of a relayed transport address
    /// for the allocation, the server MUST generate an error response with
    /// the 443 (Peer Address Family Mismatch) response code.
    ///
    /// The server MAY impose restrictions on the IP address allowed in the
    /// XOR-PEER-ADDRESS attribute; if a value is not allowed, the server
    /// rejects the request with a 403 (Forbidden) error.
    ///
    /// If the message is valid and the server is capable of carrying out the
    /// request, then the server installs or refreshes a permission for the
    /// IP address contained in each XOR-PEER-ADDRESS attribute as described
    /// in [Section 9](https://tools.ietf.org/html/rfc8656#section-9).  
    /// The port portion of each attribute is ignored and may be any arbitrary
    /// value.
    ///
    /// The server then responds with a CreatePermission success response.
    /// There are no mandatory attributes in the success response.
    ///
    /// > NOTE: A server need not do anything special to implement
    /// idempotency of CreatePermission requests over UDP using the
    /// "stateless stack approach".  Retransmitted CreatePermission
    /// requests will simply refresh the permissions.
    fn create_permission(&self, addr: &SessionAddr, username: &str, ports: &[u16]) {}

    /// refresh request
    ///
    /// If the server receives a Refresh Request with a REQUESTED-ADDRESS-
    /// FAMILY attribute and the attribute value does not match the address
    /// family of the allocation, the server MUST reply with a 443 (Peer
    /// Address Family Mismatch) Refresh error response.
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
    /// * If the "desired lifetime" is zero, then the request succeeds and
    /// the allocation is deleted.
    ///
    /// * If the "desired lifetime" is non-zero, then the request succeeds
    /// and the allocation's time-to-expiry is set to the "desired
    /// lifetime".
    ///
    /// If the request succeeds, then the server sends a success response
    /// containing:
    ///
    /// * A LIFETIME attribute containing the current value of the time-to-
    /// expiry timer.
    ///
    /// NOTE: A server need not do anything special to implement
    /// idempotency of Refresh requests over UDP using the "stateless
    /// stack approach".  Retransmitted Refresh requests with a non-
    /// zero "desired lifetime" will simply refresh the allocation.  A
    /// retransmitted Refresh request with a zero "desired lifetime"
    /// will cause a 437 (Allocation Mismatch) response if the
    /// allocation has already been deleted, but the client will treat
    /// this as equivalent to a success response (see below).
    fn refresh(&self, addr: &SessionAddr, username: &str, lifetime: u32) {}

    /// session closed
    ///
    /// Triggered when the session leaves from the turn. Possible reasons: the
    /// session life cycle has expired, external active deletion, or active
    /// exit of the session.
    fn closed(&self, addr: &SessionAddr, username: &str) {}
}

/// Turn service.
#[derive(Clone)]
pub struct Service<T> {
    interfaces: Arc<Vec<SocketAddr>>,
    sessions: Arc<Sessions<T>>,
    realm: Arc<String>,
    observer: T,
}

impl<T> Service<T>
where
    T: Clone + Observer + 'static,
{
    pub fn get_sessions(&self) -> Arc<Sessions<T>> {
        self.sessions.clone()
    }

    /// Create turn service.
    ///
    /// # Test
    ///
    /// ```
    /// use mycrl_turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {}
    ///
    /// Service::new("test".to_string(), vec![], ObserverTest);
    /// ```
    pub fn new(realm: String, interfaces: Vec<SocketAddr>, observer: T) -> Self {
        Self {
            sessions: Sessions::new(observer.clone()),
            interfaces: Arc::new(interfaces),
            realm: Arc::new(realm),
            observer,
        }
    }

    /// Get operationer.
    ///
    /// # Test
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use stun::attribute::Transport;
    /// use mycrl_turn::*;
    ///
    /// #[derive(Clone)]
    /// struct ObserverTest;
    ///
    /// impl Observer for ObserverTest {}
    ///
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let service = Service::new("test".to_string(), vec![], ObserverTest);
    ///
    /// service.get_operationer(addr, addr);
    /// ```
    pub fn get_operationer(&self, endpoint: SocketAddr, interface: SocketAddr) -> Operationer<T> {
        Operationer::new(ServiceContext {
            interfaces: self.interfaces.clone(),
            observer: self.observer.clone(),
            sessions: self.sessions.clone(),
            realm: self.realm.clone(),
            interface,
            endpoint,
        })
    }
}
