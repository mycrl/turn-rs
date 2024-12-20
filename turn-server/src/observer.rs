use std::{net::SocketAddr, sync::Arc};

use crate::{
    config::{Config, Transport},
    publicly::HooksService,
    statistics::Statistics,
};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use turn::Symbol;

#[derive(Clone)]
pub struct Observer {
    hooks: Arc<HooksService>,
    statistics: Statistics,
}

impl Observer {
    pub async fn new(config: Arc<Config>, statistics: Statistics) -> Result<Self> {
        Ok(Self {
            hooks: Arc::new(HooksService::new(config)?),
            statistics,
        })
    }
}

#[async_trait]
impl turn::Observer for Observer {
    async fn get_password(&self, key: &Symbol, name: &str) -> Option<String> {
        let pwd = self.hooks.get_password(key, name).await;

        log::info!(
            "auth: address={:?}, interface={:?}, transport={:?}, username={:?}, password={:?}",
            key.address,
            key.interface,
            key.transport,
            name,
            pwd
        );

        pwd
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
    #[allow(clippy::let_underscore_future)]
    fn allocated(&self, key: &Symbol, name: &str, port: u16) {
        log::info!(
            "allocate: address={:?}, interface={:?}, transport={:?}, username={:?}, port={}",
            key.address,
            key.interface,
            key.transport,
            name,
            port
        );

        self.statistics.register(key.address);
        self.hooks.emit(json!({
            "kind": "allocated",
            "session": {
                "address": key.address,
                "interface": key.interface,
                "transport": Transport::from(key.transport),
            },
            "username": name,
            "port": port,
        }));
    }

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
    #[allow(clippy::let_underscore_future)]
    fn channel_bind(&self, key: &Symbol, name: &str, channel: u16) {
        log::info!(
            "channel bind: address={:?}, interface={:?}, transport={:?}, username={:?}, channel={}",
            key.address,
            key.interface,
            key.transport,
            name,
            channel
        );

        self.hooks.emit(json!({
            "kind": "channel_bind",
            "session": {
                "address": key.address,
                "interface": key.interface,
                "transport": Transport::from(key.transport),
            },
            "username": name,
            "channel": channel,
        }));
    }

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
    #[allow(clippy::let_underscore_future)]
    fn create_permission(&self, key: &Symbol, name: &str, relay: &SocketAddr) {
        log::info!(
            "create permission: address={:?}, interface={:?}, transport={:?}, username={:?}, realy={:?}",
            key.address, key.interface, key.transport,
            name,
            relay
        );

        self.hooks.emit(json!({
            "kind": "create_permission",
            "session": {
                "address": key.address,
                "interface": key.interface,
                "transport": Transport::from(key.transport),
            },
            "username": name,
            "relay": relay,
        }));
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
    #[allow(clippy::let_underscore_future)]
    fn refresh(&self, key: &Symbol, name: &str, expiration: u32) {
        log::info!(
            "refresh: address={:?}, interface={:?}, transport={:?}, username={:?}, expiration={}",
            key.address,
            key.interface,
            key.transport,
            name,
            expiration
        );

        self.hooks.emit(json!({
            "kind": "refresh",
            "session": {
                "address": key.address,
                "interface": key.interface,
                "transport": Transport::from(key.transport),
            },
            "username": name,
            "expiration": expiration,
        }))
    }

    /// session closed
    ///
    /// Triggered when the session leaves from the turn. Possible reasons: the
    /// session life cycle has expired, external active deletion, or active
    /// exit of the session.
    #[allow(clippy::let_underscore_future)]
    fn closed(&self, key: &Symbol, name: &str) {
        log::info!(
            "closed: address={:?}, interface={:?}, transport={:?}, username={:?}",
            key.address,
            key.interface,
            key.transport,
            name
        );

        self.statistics.unregister(&key.address);
        self.hooks.emit(json!({
            "kind": "closed",
            "session": {
                "address": key.address,
                "interface": key.interface,
                "transport": Transport::from(key.transport),
            },
            "username": name,
        }))
    }
}
