//! # Turn Server SDK
//!
//! A Rust client SDK for interacting with the `turn-server` gRPC API exposed by the `turn-rs` project.
//! This crate provides both client and server utilities for TURN server integration.
//!
//! ## Features
//!
//! - **TurnService Client**: Query server information, session details, and manage TURN sessions
//! - **TurnHooksServer**: Implement custom authentication and event handling for TURN server hooks
//! - **Password Generation**: Generate STUN/TURN authentication passwords using MD5 or SHA256
//!
//! ## Client Usage
//!
//! The `TurnService` client allows you to interact with a running TURN server's gRPC API:
//!
//! ```no_run
//! use turn_server_sdk::{TurnService, tonic::transport::Channel};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to the TURN server gRPC endpoint
//! let channel = Channel::from_static("http://127.0.0.1:3000")
//!     .connect()
//!     .await?;
//!
//! // Create a client
//! let mut client = TurnService::new(channel);
//!
//! // Get server information
//! let info = client.get_info().await?;
//! println!("Server software: {}", info.software);
//!
//! // Query a session by ID
//! let session = client.get_session("session-id".to_string()).await?;
//! println!("Session username: {}", session.username);
//!
//! // Get session statistics
//! let stats = client.get_session_statistics("session-id".to_string()).await?;
//! println!("Bytes sent: {}", stats.send_bytes);
//!
//! // Destroy a session
//! client.destroy_session("session-id".to_string()).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Server Usage (Hooks Implementation)
//!
//! Implement the `TurnHooksServer` trait to provide custom authentication and handle TURN events:
//!
//! ```no_run
//! use turn_server_sdk::{
//!     TurnHooksServer, Credential, protos::PasswordAlgorithm,
//!     tonic::transport::Server
//! };
//!
//! use std::net::SocketAddr;
//!
//! struct MyHooksServer;
//!
//! #[tonic::async_trait]
//! impl TurnHooksServer for MyHooksServer {
//!     async fn get_password(
//!         &self,
//!         realm: &str,
//!         username: &str,
//!         algorithm: PasswordAlgorithm,
//!     ) -> Result<Credential, tonic::Status> {
//!         // Implement your authentication logic here
//!         // For example, look up the user in a database
//!         Ok(Credential {
//!             password: "user-password".to_string(),
//!             realm: realm.to_string(),
//!         })
//!     }
//!
//!     async fn on_allocated(&self, id: String, username: String, port: u16) {
//!         println!("Session allocated: id={}, username={}, port={}", id, username, port);
//!         // Handle allocation event (e.g., log to database, update metrics)
//!     }
//!
//!     async fn on_destroy(&self, id: String, username: String) {
//!         println!("Session destroyed: id={}, username={}", id, username);
//!         // Handle session destruction (e.g., cleanup resources)
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Start the hooks server
//! let mut server = Server::builder();
//! let hooks = MyHooksServer;
//!
//! hooks.start_with_server(
//!     &mut server,
//!     "127.0.0.1:8080".parse()?,
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Password Generation
//!
//! Generate STUN/TURN authentication passwords for long-term credentials:
//!
//! ```no_run
//! use turn_server_sdk::{generate_password, protos::PasswordAlgorithm};
//!
//! // Generate MD5 password (RFC 5389)
//! let md5_password = generate_password(
//!     "username",
//!     "password",
//!     "realm",
//!     PasswordAlgorithm::Md5,
//! );
//!
//! // Generate SHA256 password (RFC 8489)
//! let sha256_password = generate_password(
//!     "username",
//!     "password",
//!     "realm",
//!     PasswordAlgorithm::Sha256,
//! );
//!
//! // Access the password bytes
//! match md5_password {
//!     turn_server_sdk::Password::Md5(bytes) => {
//!         println!("MD5 password: {:?}", bytes);
//!     }
//!     turn_server_sdk::Password::Sha256(bytes) => {
//!         println!("SHA256 password: {:?}", bytes);
//!     }
//! }
//! ```
//!
//! ## Event Handling
//!
//! The `TurnHooksServer` trait provides hooks for various TURN server events:
//!
//! - `on_allocated`: Called when a client allocates a relay port
//! - `on_channel_bind`: Called when a channel is bound to a peer
//! - `on_create_permission`: Called when permissions are created for peers
//! - `on_refresh`: Called when a session is refreshed
//! - `on_destroy`: Called when a session is destroyed
//!
//! All event handlers are optional and have default no-op implementations.
//!
//! ## Error Handling
//!
//! Most operations return `Result<T, Status>` where `Status` is a gRPC status code.
//! Common error scenarios:
//!
//! - `Status::not_found`: Session or resource not found
//! - `Status::unavailable`: Server is not available
//! - `Status::unauthenticated`: Authentication failed
//!
//! ## Re-exports
//!
//! This crate re-exports:
//! - `tonic`: The gRPC framework used for communication
//! - `protos`: The generated protobuf bindings for TURN server messages
//!
//! ## See Also
//!
//! - [TURN Server Documentation](../README.md)
//! - [RFC 8489](https://tools.ietf.org/html/rfc8489) - Session Traversal Utilities for NAT (STUN)
//! - [RFC 8656](https://tools.ietf.org/html/rfc8656) - Traversal Using Relays around NAT (TURN)

pub use protos;
pub use tonic;

use std::{net::SocketAddr, ops::Deref};

use aws_lc_rs::digest;
use md5::{Digest, Md5};
use tonic::{
    Request, Response, Status,
    transport::{Channel, Server},
};

use protos::{
    GetTurnPasswordRequest, GetTurnPasswordResponse, PasswordAlgorithm, SessionQueryParams,
    TurnAllocatedEvent, TurnChannelBindEvent, TurnCreatePermissionEvent, TurnDestroyEvent,
    TurnRefreshEvent, TurnServerInfo, TurnSession, TurnSessionStatistics,
    turn_hooks_service_server::{TurnHooksService, TurnHooksServiceServer},
    turn_service_client::TurnServiceClient,
};

/// turn service client
///
/// This struct is used to interact with the turn service.
pub struct TurnService(TurnServiceClient<Channel>);

impl TurnService {
    /// create a new turn service client
    pub fn new(channel: Channel) -> Self {
        Self(TurnServiceClient::new(channel))
    }

    /// get the server info
    pub async fn get_info(&mut self) -> Result<TurnServerInfo, Status> {
        Ok(self.0.get_info(Request::new(())).await?.into_inner())
    }

    /// get the session
    pub async fn get_session(&mut self, id: String) -> Result<TurnSession, Status> {
        Ok(self
            .0
            .get_session(Request::new(SessionQueryParams { id }))
            .await?
            .into_inner())
    }

    /// get the session statistics
    pub async fn get_session_statistics(
        &mut self,
        id: String,
    ) -> Result<TurnSessionStatistics, Status> {
        Ok(self
            .0
            .get_session_statistics(Request::new(SessionQueryParams { id }))
            .await?
            .into_inner())
    }

    /// destroy the session
    pub async fn destroy_session(&mut self, id: String) -> Result<(), Status> {
        Ok(self
            .0
            .destroy_session(Request::new(SessionQueryParams { id }))
            .await?
            .into_inner())
    }
}

/// credential
///
/// This struct is used to store the credential for the turn hooks server.
pub struct Credential {
    pub password: String,
    pub realm: String,
}

struct TurnHooksServerInner<T>(T);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Password {
    Md5([u8; 16]),
    Sha256([u8; 32]),
}

impl Deref for Password {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            Password::Md5(it) => it,
            Password::Sha256(it) => it,
        }
    }
}

pub fn generate_password(
    username: &str,
    password: &str,
    realm: &str,
    algorithm: PasswordAlgorithm,
) -> Password {
    match algorithm {
        PasswordAlgorithm::Md5 => {
            let mut hasher = Md5::new();

            hasher.update([username, realm, password].join(":"));

            Password::Md5(hasher.finalize().into())
        }
        PasswordAlgorithm::Sha256 => {
            let mut ctx = digest::Context::new(&digest::SHA256);

            ctx.update([username, realm, password].join(":").as_bytes());

            let mut result = [0u8; 32];
            result.copy_from_slice(ctx.finish().as_ref());
            Password::Sha256(result)
        }
        PasswordAlgorithm::Unspecified => {
            panic!("Invalid password algorithm");
        }
    }
}

#[tonic::async_trait]
impl<T: TurnHooksServer + 'static> TurnHooksService for TurnHooksServerInner<T> {
    async fn get_password(
        &self,
        request: Request<GetTurnPasswordRequest>,
    ) -> Result<Response<GetTurnPasswordResponse>, Status> {
        let request = request.into_inner();
        let algorithm = request.algorithm();

        if let Ok(credential) = self
            .0
            .get_password(&request.realm, &request.username, algorithm)
            .await
        {
            Ok(Response::new(GetTurnPasswordResponse {
                password: generate_password(
                    &request.username,
                    &credential.password,
                    &credential.realm,
                    algorithm,
                )
                .to_vec(),
            }))
        } else {
            Err(Status::not_found("Message integrity not found"))
        }
    }

    async fn on_allocated_event(
        &self,
        request: Request<TurnAllocatedEvent>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        self.0
            .on_allocated(request.id, request.username, request.port as u16)
            .await;

        Ok(Response::new(()))
    }

    async fn on_channel_bind_event(
        &self,
        request: Request<TurnChannelBindEvent>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        self.0
            .on_channel_bind(request.id, request.username, request.channel as u16)
            .await;

        Ok(Response::new(()))
    }

    async fn on_create_permission_event(
        &self,
        request: Request<TurnCreatePermissionEvent>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        self.0
            .on_create_permission(
                request.id,
                request.username,
                request.ports.iter().map(|p| *p as u16).collect(),
            )
            .await;

        Ok(Response::new(()))
    }

    async fn on_refresh_event(
        &self,
        request: Request<TurnRefreshEvent>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        self.0
            .on_refresh(request.id, request.username, request.lifetime as u32)
            .await;

        Ok(Response::new(()))
    }

    async fn on_destroy_event(
        &self,
        request: Request<TurnDestroyEvent>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        self.0.on_destroy(request.id, request.username).await;

        Ok(Response::new(()))
    }
}

#[tonic::async_trait]
pub trait TurnHooksServer: Send + Sync {
    #[allow(unused_variables)]
    async fn get_password(
        &self,
        realm: &str,
        username: &str,
        algorithm: PasswordAlgorithm,
    ) -> Result<Credential, Status> {
        Err(Status::unimplemented("get_password is not implemented"))
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
    #[allow(unused_variables)]
    async fn on_allocated(&self, id: String, username: String, port: u16) {}

    /// channel bind request
    ///
    /// [rfc8489](https://tools.ietf.org/html/rfc8489)
    ///
    /// If the request is valid, but the server is unable to fulfill the
    /// request due to some capacity limit or similar, the server replies
    /// with a 508 (Insufficient Capacity) error.
    ///
    /// Otherwise, the server replies with a ChannelBind success response.
    /// There are no required attributes in a successful ChannelBind
    /// response.
    #[allow(unused_variables)]
    async fn on_channel_bind(&self, id: String, username: String, channel: u16) {}

    /// create permission request
    ///
    /// [rfc8489](https://tools.ietf.org/html/rfc8489)
    ///
    /// If the request is valid, but the server is unable to fulfill the
    /// request due to some capacity limit or similar, the server replies
    /// with a 508 (Insufficient Capacity) error.
    ///
    /// Otherwise, the server replies with a ChannelBind success response.
    /// There are no required attributes in a successful ChannelBind
    /// response.
    #[allow(unused_variables)]
    async fn on_create_permission(&self, id: String, username: String, ports: Vec<u16>) {}

    /// refresh request
    ///
    /// [rfc8489](https://tools.ietf.org/html/rfc8489)
    ///
    /// If the request is valid, but the server is unable to fulfill the
    /// request due to some capacity limit or similar, the server replies
    /// with a 508 (Insufficient Capacity) error.
    ///
    /// Otherwise, the server replies with a ChannelBind success response.
    /// There are no required attributes in a successful ChannelBind
    /// response.
    #[allow(unused_variables)]
    async fn on_refresh(&self, id: String, username: String, lifetime: u32) {}

    /// session closed
    ///
    /// Triggered when the session leaves from the turn. Possible reasons: the
    /// session life cycle has expired, external active deletion, or active
    /// exit of the session.
    #[allow(unused_variables)]
    async fn on_destroy(&self, id: String, username: String) {}

    /// start the turn hooks server
    ///
    /// This function will start the turn hooks server on the given server and listen address.
    async fn start_with_server(
        self,
        server: &mut Server,
        listen: SocketAddr,
    ) -> Result<(), tonic::transport::Error>
    where
        Self: Sized + 'static,
    {
        server
            .add_service(TurnHooksServiceServer::<TurnHooksServerInner<Self>>::new(
                TurnHooksServerInner(self),
            ))
            .serve(listen)
            .await?;

        Ok(())
    }
}
