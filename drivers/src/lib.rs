use std::{fmt::Display, future::Future, net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    extract::{Json as Body, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use reqwest::{Client, ClientBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    TCP,
    UDP,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionAddr {
    pub address: SocketAddr,
    pub interface: SocketAddr,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Interface {
    pub transport: Transport,
    /// turn server listen address
    pub bind: SocketAddr,
    /// specify the node external address and port
    pub external: SocketAddr,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Info {
    /// Software information of turn server
    pub software: String,
    /// Turn the server's running time in seconds
    pub uptime: u64,
    /// The number of allocated ports
    pub port_allocated: u16,
    /// The total number of ports available for allocation
    pub port_capacity: u16,
    /// Turn all interfaces bound to the server
    pub interfaces: Vec<Interface>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Session {
    /// The IP address and port number currently used by the session
    pub address: SocketAddr,
    /// Username used in session authentication
    pub username: String,
    /// The password used in session authentication
    pub password: String,
    /// Channel numbers that have been assigned to the session
    pub channels: Vec<u16>,
    /// Port numbers that have been assigned to the session
    pub port: Option<u16>,
    /// The validity period of the current session application, in seconds
    pub expires: u32,
    pub permissions: Vec<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Statistics {
    /// Number of bytes received in the current session
    pub received_bytes: u64,
    /// The number of bytes sent by the current session
    pub send_bytes: u64,
    /// Number of packets received in the current session
    pub received_pkts: u64,
    /// The number of packets sent by the current session
    pub send_pkts: u64,
    /// The number of packets error by the current session
    pub error_pkts: u64,
}

impl<'a> Display for SessionAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            format!("address={}&interface={}", self.address, self.interface)
        )
    }
}

/// Controlling message packaging
#[derive(Debug)]
pub struct Message<T> {
    /// turn server realm
    pub realm: String,
    /// The runtime id of the turn server. A new ID is generated each time the
    /// server is started. This is a random string. Its main function is to
    /// determine whether the turn server has been restarted.
    pub nonce: String,
    pub payload: T,
}

impl<T> Message<T> {
    async fn from_res<F: Future<Output = Option<T>>>(
        res: Response,
        handler: impl FnOnce(Response) -> F,
    ) -> Option<Self> {
        let (realm, nonce) = get_realm_and_nonce(res.headers())?;
        Some(Self {
            realm: realm.to_string(),
            nonce: nonce.to_string(),
            payload: handler(res).await?,
        })
    }
}

/// The controller of the turn server is used to control the server and obtain
/// server information through the HTTP interface
pub struct Controller {
    client: Client,
    server: String,
}

impl Controller {
    /// Create a controller by specifying the listening address of the turn
    /// server api interface, such as `http://localhost:3000`
    pub fn new(server: &str) -> Result<Self, reqwest::Error> {
        Ok(Self {
            server: server.to_string(),
            client: ClientBuilder::new()
                .timeout(Duration::from_secs(5))
                .build()?,
        })
    }

    /// Get the information of the turn server, including version information,
    /// listening interface, startup time, etc.
    pub async fn get_info(&self) -> Option<Message<Info>> {
        Message::from_res(
            self.client
                .get(format!("{}/info", self.server))
                .send()
                .await
                .ok()?,
            |res| async { res.json().await.ok() },
        )
        .await
    }

    /// Get session information. A session corresponds to each UDP socket. It
    /// should be noted that a user can have multiple sessions at the same time.
    pub async fn get_session(&self, query: &SessionAddr) -> Option<Message<Session>> {
        Message::from_res(
            self.client
                .get(format!("{}/session?{}", self.server, query))
                .send()
                .await
                .ok()?,
            |res| async { res.json().await.ok() },
        )
        .await
    }

    /// Get session statistics, which is mainly the traffic statistics of the
    /// current session
    pub async fn get_session_statistics(&self, query: &SessionAddr) -> Option<Message<Statistics>> {
        Message::from_res(
            self.client
                .get(format!("{}/session/statistics?{}", self.server, query))
                .send()
                .await
                .ok()?,
            |res| async { res.json().await.ok() },
        )
        .await
    }

    /// Delete the session. Deleting the session will cause the turn server to
    /// delete all routing information of the current session. If there is a
    /// peer, the peer will also be disconnected.
    pub async fn remove_session(&self, query: &SessionAddr) -> Option<Message<bool>> {
        Message::from_res(
            self.client
                .delete(format!("{}/session?{}", self.server, query))
                .send()
                .await
                .ok()?,
            |res| async move { Some(res.status() == StatusCode::OK) },
        )
        .await
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Events {
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
    Allocated {
        session: SessionAddr,
        username: String,
        port: u16,
    },
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
    ChannelBind {
        session: SessionAddr,
        username: String,
        channel: u16,
    },
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
    /// NOTE: A server need not do anything special to implement idempotency of
    /// CreatePermission requests over UDP using the "stateless stack approach".
    /// Retransmitted CreatePermission requests will simply refresh the
    /// permissions.
    CreatePermission {
        session: SessionAddr,
        username: String,
        ports: Vec<u16>,
    },
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
    /// * If the "desired lifetime" is zero, then the request succeeds and the
    ///   allocation is deleted.
    ///
    /// * If the "desired lifetime" is non-zero, then the request succeeds and
    ///   the allocation's time-to-expiry is set to the "desired lifetime".
    ///
    /// If the request succeeds, then the server sends a success response
    /// containing:
    ///
    /// * A LIFETIME attribute containing the current value of the
    ///   time-to-expiry timer.
    ///
    /// NOTE: A server need not do anything special to implement
    /// idempotency of Refresh requests over UDP using the "stateless
    /// stack approach".  Retransmitted Refresh requests with a non-
    /// zero "desired lifetime" will simply refresh the allocation.  A
    /// retransmitted Refresh request with a zero "desired lifetime"
    /// will cause a 437 (Allocation Mismatch) response if the
    /// allocation has already been deleted, but the client will treat
    /// this as equivalent to a success response (see below).
    Refresh {
        session: SessionAddr,
        username: String,
        lifetime: u32,
    },
    /// session closed
    ///
    /// Triggered when the session leaves from the turn. Possible reasons: the
    /// session life cycle has expired, external active deletion, or active
    /// exit of the session.
    Closed {
        session: SessionAddr,
        username: String,
    },
}

/// Abstraction that handles turn server communication with the outside world
///
/// ```ignore
/// struct HooksImpl;
///
/// #[async_trait]
/// impl Hooks for HooksImpl {
///     async fn auth(&self, addr: SocketAddr, name: String, realm: String, rid: String) -> Option<&str> {
///         get_password(username).await // Pretend this function exists
///     }
///
///     async fn on(&self, event: Events, realm: String, rid: String) {
///         println!("event={:?}, realm={}, rid={}", event, realm, rid)
///     }
/// }
/// ```
#[async_trait]
pub trait Hooks {
    /// When the turn server needs to authenticate the current user, hooks only
    /// needs to find the key according to the username and other information of
    /// the current session and return it
    #[allow(unused_variables)]
    async fn auth(
        &self,
        session: &SessionAddr,
        username: &str,
        realm: &str,
        nonce: &str,
    ) -> Option<&str> {
        None
    }

    /// Called when the turn server pushes an event
    #[allow(unused_variables)]
    async fn on(&self, event: &Events, realm: &str, nonce: &str) {}
}

#[derive(Deserialize)]
struct GetPasswordQuery {
    address: SocketAddr,
    interface: SocketAddr,
    username: String,
}

/// Create a hooks service, which will create an HTTP server. The turn server
/// can request this server and push events to this server.
pub async fn start_hooks_server<T>(bind: SocketAddr, hooks: T) -> Result<(), std::io::Error>
where
    T: Hooks + Send + Sync + 'static,
{
    let app = Router::new()
        .route(
            "/password",
            get(
                |headers: HeaderMap,
                 State(state): State<Arc<T>>,
                 Query(query): Query<GetPasswordQuery>| async move {
                    if let Some((realm, nonce)) = get_realm_and_nonce(&headers) {
                        if let Some(password) =
                            state.auth(&SessionAddr {
                                address: query.address,
                                interface: query.interface,
                            }, &query.username, realm, nonce).await
                        {
                            return password.to_string().into_response();
                        }
                    }

                    StatusCode::NOT_FOUND.into_response()
                },
            ),
        )
        .route(
            "/events",
            post(
                |headers: HeaderMap, State(state): State<Arc<T>>, Body(event): Body<Events>| async move {
                    if let Some((realm, nonce)) = get_realm_and_nonce(&headers) {
                        state.on(&event, realm, nonce).await;
                    }

                    StatusCode::OK
                },
            ),
        )
        .with_state(Arc::new(hooks));

    axum::serve(TcpListener::bind(bind).await?, app).await?;

    Ok(())
}

fn get_realm_and_nonce(headers: &HeaderMap) -> Option<(&str, &str)> {
    if let (Some(Ok(realm)), Some(Ok(nonce))) = (
        headers.get("realm").map(|it| it.to_str()),
        headers.get("nonce").map(|it| it.to_str()),
    ) {
        Some((realm, nonce))
    } else {
        None
    }
}
