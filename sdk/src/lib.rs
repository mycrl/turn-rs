pub use tonic;

pub mod proto {
    tonic::include_proto!("turn.server");
}

use std::net::SocketAddr;

pub use codec::message::attributes::PasswordAlgorithm;

use tonic::{
    Request, Response, Status,
    transport::{Channel, Server},
};

use crate::proto::{
    GetTurnPasswordRequest, GetTurnPasswordResponse, SessionQueryParams, TurnAllocatedEvent,
    TurnChannelBindEvent, TurnCreatePermissionEvent, TurnDestroyEvent, TurnRefreshEvent,
    TurnServerInfo, TurnSession, TurnSessionStatistics,
    turn_hooks_service_server::{TurnHooksService, TurnHooksServiceServer},
    turn_service_client::TurnServiceClient,
};

impl TryInto<PasswordAlgorithm> for proto::PasswordAlgorithm {
    type Error = Status;

    fn try_into(self) -> Result<PasswordAlgorithm, Self::Error> {
        Ok(match self {
            proto::PasswordAlgorithm::Md5 => PasswordAlgorithm::Md5,
            proto::PasswordAlgorithm::Sha256 => PasswordAlgorithm::Sha256,
            proto::PasswordAlgorithm::Unspecified => {
                return Err(Status::invalid_argument("Invalid password algorithm"));
            }
        })
    }
}

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
pub struct Credential<'a> {
    pub password: &'a str,
    pub realm: &'a str,
}

struct TurnHooksServerInner<T>(T);

#[tonic::async_trait]
impl<T: TurnHooksServer + 'static> TurnHooksService for TurnHooksServerInner<T> {
    async fn get_password(
        &self,
        request: Request<GetTurnPasswordRequest>,
    ) -> Result<Response<GetTurnPasswordResponse>, Status> {
        let request = request.into_inner();
        let algorithm = request.algorithm().try_into()?;

        if let Ok(credential) = self.0.get_password(&request.username, algorithm).await {
            Ok(Response::new(GetTurnPasswordResponse {
                password: codec::crypto::generate_password(
                    &request.username,
                    credential.password,
                    credential.realm,
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
