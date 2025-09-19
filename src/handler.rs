#[cfg(feature = "rpc")]
use std::sync::Arc;

use crate::{config::Config, statistics::Statistics};

#[cfg(feature = "rpc")]
use crate::rpc::{
    HooksEvent, IdString, RpcHooksService,
    proto::{
        TurnAllocatedEvent, TurnChannelBindEvent, TurnCreatePermissionEvent, TurnDestroyEvent,
        TurnRefreshEvent,
    },
};

use anyhow::Result;
use codec::{crypto::Password, message::attributes::PasswordAlgorithm};
use service::{ServiceHandler, session::Identifier};

#[derive(Clone)]
pub struct Handler {
    config: Config,
    #[cfg(feature = "rpc")]
    statistics: Statistics,
    #[cfg(feature = "rpc")]
    rpc: Arc<RpcHooksService>,
}

impl Handler {
    #[allow(unused_variables)]
    pub async fn new(config: Config, statistics: Statistics) -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "rpc")]
            rpc: RpcHooksService::new(&config).await?.into(),
            #[cfg(feature = "rpc")]
            statistics,
            config,
        })
    }
}

impl ServiceHandler for Handler {
    async fn get_password(&self, username: &str, algorithm: PasswordAlgorithm) -> Option<Password> {
        // Match the static authentication information first.
        if let Some(password) = self.config.auth.static_credentials.get(username) {
            return Some(codec::crypto::generate_password(
                username,
                password,
                &self.config.server.realm,
                algorithm,
            ));
        }

        // Try again to match the static authentication key.
        if let Some(secret) = &self.config.auth.static_auth_secret {
            return Some(codec::crypto::static_auth_secret(
                username,
                &secret,
                &self.config.server.realm,
                algorithm,
            ));
        }

        #[cfg(feature = "rpc")]
        if self.config.auth.enable_hooks_auth {
            return self.rpc.get_password(username, algorithm).await;
        }

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
    fn on_allocated(&self, id: &Identifier, name: &str, port: u16) {
        log::info!(
            "allocate: address={:?}, interface={:?}, username={:?}, port={}",
            id.source,
            id.interface,
            name,
            port
        );

        #[cfg(feature = "rpc")]
        {
            self.statistics.register(*id);

            self.rpc
                .send_event(HooksEvent::Allocated(TurnAllocatedEvent {
                    id: id.to_string(),
                    username: name.to_string(),
                    port: port as i32,
                }));
        }
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
    /// CHANNEL-NUMBER attribute and the interface address in the XOR-PEER-
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
    fn on_channel_bind(&self, id: &Identifier, name: &str, channel: u16) {
        log::info!(
            "channel bind: address={:?}, interface={:?}, username={:?}, channel={}",
            id.source,
            id.interface,
            name,
            channel
        );

        #[cfg(feature = "rpc")]
        {
            self.rpc
                .send_event(HooksEvent::ChannelBind(TurnChannelBindEvent {
                    id: id.to_string(),
                    username: name.to_string(),
                    channel: channel as i32,
                }));
        }
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
    /// family that is not the same as that of a relayed interface address
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
    fn on_create_permission(&self, id: &Identifier, name: &str, ports: &[u16]) {
        log::info!(
            "create permission: address={:?}, interface={:?}, username={:?}, ports={:?}",
            id.source,
            id.interface,
            name,
            ports
        );

        #[cfg(feature = "rpc")]
        {
            self.rpc
                .send_event(HooksEvent::CreatePermission(TurnCreatePermissionEvent {
                    id: id.to_string(),
                    username: name.to_string(),
                    ports: ports.iter().map(|p| *p as i32).collect(),
                }));
        }
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
    fn on_refresh(&self, id: &Identifier, name: &str, lifetime: u32) {
        log::info!(
            "refresh: address={:?}, interface={:?}, username={:?}, lifetime={}",
            id.source,
            id.interface,
            name,
            lifetime
        );

        #[cfg(feature = "rpc")]
        {
            self.rpc.send_event(HooksEvent::Refresh(TurnRefreshEvent {
                id: id.to_string(),
                username: name.to_string(),
                lifetime: lifetime as i32,
            }));
        }
    }

    /// session closed
    ///
    /// Triggered when the session leaves from the turn. Possible reasons: the
    /// session life cycle has expired, external active deletion, or active
    /// exit of the session.
    fn on_destroy(&self, id: &Identifier, name: &str) {
        log::info!(
            "closed: address={:?}, interface={:?}, username={:?}",
            id.source,
            id.interface,
            name
        );

        #[cfg(feature = "rpc")]
        {
            self.statistics.unregister(&id);

            self.rpc.send_event(HooksEvent::Destroy(TurnDestroyEvent {
                id: id.to_string(),
                username: name.to_string(),
            }));
        }
    }
}
