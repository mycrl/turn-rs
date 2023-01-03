pub mod processor;
pub mod router;

pub use processor::Processor;
pub use router::nodes::Node;
pub use router::Router;

use async_trait::async_trait;
use std::{
    net::SocketAddr,
    sync::Arc,
};

#[async_trait]
pub trait Observer: Send + Sync {
    /// turn auth request
    #[allow(unused)]
    async fn auth(&self, addr: &SocketAddr, name: &str) -> Option<String> {
        None
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
    #[allow(unused)]
    fn allocated(&self, addr: &SocketAddr, name: &str, port: u16) {}

    /// binding request
    ///
    /// [rfc8489](https://tools.ietf.org/html/rfc8489)
    ///
    /// In the Binding request/response transaction, a Binding request is
    /// sent from a STUN client to a STUN server.  When the Binding request
    /// arrives at the STUN server, it may have passed through one or more
    /// NATs between the STUN client and the STUN server (in Figure 1, there
    /// are two such NATs).  As the Binding request message passes through a
    /// NAT, the NAT will modify the source transport address (that is, the
    /// source IP address and the source port) of the packet.  As a result,
    /// the source transport address of the request received by the server
    /// will be the public IP address and port created by the NAT closest to
    /// the server.  This is called a "reflexive transport address".  The
    /// STUN server copies that source transport address into an XOR-MAPPED-
    /// ADDRESS attribute in the STUN Binding response and sends the Binding
    /// response back to the STUN client.  As this packet passes back through
    /// a NAT, the NAT will modify the destination transport address in the
    /// IP header, but the transport address in the XOR-MAPPED-ADDRESS
    /// attribute within the body of the STUN response will remain untouched.
    /// In this way, the client can learn its reflexive transport address
    /// allocated by the outermost NAT with respect to the STUN server.
    #[allow(unused)]
    fn binding(&self, addr: &SocketAddr) {}

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
    #[allow(unused)]
    fn channel_bind(&self, addr: &SocketAddr, name: &str, num: u16) {}

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
    #[allow(unused)]
    fn create_permission(
        &self,
        addr: &SocketAddr,
        name: &str,
        relay: &SocketAddr,
    ) {
    }

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
    #[allow(unused)]
    fn refresh(&self, addr: &SocketAddr, name: &str, time: u32) {}
}

/// Service options.
pub struct Options {
    /// specify the node external address and port.
    /// for the case of exposing the service to the outside,
    /// you need to manually specify the server external IP
    /// address and service listening port.
    pub external: SocketAddr,
    /// specify the domain where the server is located.
    /// for a single node, this configuration is fixed,
    /// but each node can be configured as a different domain.
    /// this is a good idea to divide the nodes by namespace.
    pub realm: String,
}

/// TUTN service.
pub struct Service {
    router: Arc<Router>,
    observer: Arc<dyn Observer>,
    opt: Arc<Options>,
}

impl Service {
    pub fn get_router(&self) -> Arc<Router> {
        self.router.clone()
    }

    /// Create turn service.
    ///
    /// # Examples
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
    /// let _service = Service::new(
    ///     Options {
    ///         external: "127.0.0.1:4378".parse().unwrap(),
    ///         realm: "test".to_string(),
    ///     },
    ///     Events {},
    /// );
    /// ```
    pub fn new<T>(options: Options, observer: T) -> Self
    where
        T: Observer + 'static,
    {
        let opt = Arc::new(options);
        let observer = Arc::new(observer);
        let router = Router::new(opt.clone(), observer.clone());

        Self {
            observer,
            router,
            opt,
        }
    }

    /// Start run service.
    ///
    /// # Examples
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
    /// let service = Service::new(
    ///     Options {
    ///         external: "127.0.0.1:4378".parse().unwrap(),
    ///         realm: "test".to_string(),
    ///     },
    ///     Events {},
    /// );
    ///
    /// tokio::spawn(service.run()).await;
    /// ```
    pub async fn run(self) {
        self.router.start_poll().await
    }

    /// Get processor.
    ///
    /// # Examples
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
    /// let service = Service::new(
    ///     Options {
    ///         external: "127.0.0.1:4378".parse().unwrap(),
    ///         realm: "test".to_string(),
    ///     },
    ///     Events {},
    /// );
    ///
    /// tokio::spawn(service.run());
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:34254")?;
    /// let mut processor = service.get_processor();
    ///
    /// let mut buf = [0; 4096];
    /// let (amt, src) = socket.recv_from(&mut buf).unwrap();
    /// if let Ok(Some((buf, target))) =
    ///     processor.process(&mut buf[..amt], src).await
    /// {
    ///     socket.send_to(buf, target.as_ref()).unwrap();
    /// }
    /// ```
    pub fn get_processor(&self) -> Processor {
        Processor::new(
            self.opt.clone(),
            self.router.clone(),
            self.observer.clone(),
        )
    }
}
