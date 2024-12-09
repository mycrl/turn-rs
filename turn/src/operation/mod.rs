pub mod allocate;
pub mod binding;
pub mod channel_bind;
pub mod channel_data;
pub mod create_permission;
pub mod indication;
pub mod refresh;

use self::allocate::Allocate;
use crate::{state::State, Observer, StunClass};

use std::{net::SocketAddr, sync::Arc};

use bytes::BytesMut;
use stun::{
    attribute::{ErrKind, Error, ErrorCode, Realm, UserName},
    Decoder, Kind, MessageReader, MessageWriter, Method, Payload, StunError,
};

/// Return early with an error if a condition is not satisfied.
///
/// This macro is equivalent to if !$cond { return Err(RouterError); }.
///
/// The surrounding function’s or closure’s return value is required to be
/// Result<_, RouterError>.
///
/// Analogously to assert!, ensure! takes a condition and exits the function if
/// the condition fails. Unlike assert!, ensure! returns an Error rather than
/// panicking.
///
/// ```ignore
/// ensure!(1 == 0, Unauthorized);
/// ```
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err(RouterError::Reject($err));
        }
    };
}

/// The option version of the ensure macro.
///
/// Return early with an error if a condition is not satisfied.
///
/// This macro is equivalent to if !$cond { return Err(RouterError); }.
///
/// The surrounding function’s or closure’s return value is required to be
/// Result<_, RouterError>.
///
/// Analogously to assert!, ensure! takes a condition and exits the function if
/// the condition fails. Unlike assert!, ensure! returns an Error rather than
/// panicking.
///
/// ```ignore
/// ensure_optional!(None, Unauthorized);
/// ```
#[macro_export]
macro_rules! ensure_optional {
    ($cond:expr, $err:expr) => {
        if let Some(it) = $cond {
            it
        } else {
            return Err(RouterError::Reject($err));
        }
    };
}

/// The context of the service.
///
/// A service corresponds to a Net Socket, different sockets have different
/// addresses and so on, but other things are basically the same.
pub struct ServiceContext<T: Observer> {
    pub interface: SocketAddr,
    pub realm: Arc<String>,
    pub state: Arc<State<T>>,
    pub external: SocketAddr,
    pub externals: Arc<Vec<SocketAddr>>,
    pub observer: T,
}

/// Authentication information for the message.
///
/// Note that `key` is not a password, but a long-term key that has been
/// digested.
///
/// > key = MD5(username ":" OpaqueString(realm) ":" OpaqueString(password))
pub struct Auth<'a> {
    pub username: &'a str,
    pub key: Arc<[u8; 16]>,
}

pub struct Requet<'a, T, M>
where
    T: Observer + 'static,
{
    pub auth: Option<Auth<'a>>,
    pub address: SocketAddr,
    pub service: &'a ServiceContext<T>,
    pub message: &'a M,
}

impl<'a, T, M> Requet<'a, T, M>
where
    T: Observer + 'static,
{
    #[inline(always)]
    pub(crate) fn ip_is_local(&self, addr: &SocketAddr) -> bool {
        self.service
            .externals
            .iter()
            .any(|item| item.ip() == addr.ip())
    }
}

impl<'a, T> Requet<'a, T, MessageReader<'a, 'a>>
where
    T: Observer + 'static,
{
    #[inline(always)]
    pub(crate) fn create_message(
        &self,
        bytes: &'a mut BytesMut,
    ) -> Result<MessageWriter<'a>, RouterError> {
        Ok(MessageWriter::extend(
            self.message
                .method
                .into_response()
                .ok_or_else(|| RouterError::Other(StunError::InvalidInput))?,
            &self.message,
            bytes,
        ))
    }

    #[inline(always)]
    pub(crate) fn create_response(
        &self,
        mut message: MessageWriter<'a>,
    ) -> Result<Response<'a>, RouterError> {
        let auth = ensure_optional!(&self.auth, ErrKind::Unauthorized);
        message
            .flush(Some(&auth.key))
            .map_err(|e| RouterError::Other(e))?;

        Ok(Response {
            kind: StunClass::Message,
            bytes: message.bytes,
            relay: None,
        })
    }
}

#[derive(Clone, Copy)]
pub struct ResponseRelay {
    pub address: SocketAddr,
    pub interface: SocketAddr,
}

pub struct Response<'a> {
    pub kind: StunClass,
    pub bytes: &'a [u8],
    pub relay: Option<ResponseRelay>,
}

pub enum RouterError {
    Reject(ErrKind),
    Other(StunError),
}

pub trait MessageRouter<'a, T>
where
    T: Observer + 'static,
{
    const AUTH: bool;

    fn handle(
        bytes: &'a mut BytesMut,
        req: Requet<'a, T, MessageReader<'a, 'a>>,
    ) -> Result<Option<Response<'a>>, RouterError>;

    #[allow(async_fn_in_trait)]
    async fn route(
        bytes: &'a mut BytesMut,
        mut req: Requet<'a, T, MessageReader<'a, 'a>>,
    ) -> Result<Option<Response<'a>>, RouterError> {
        if Self::AUTH {
            let username = ensure_optional!(req.message.get::<UserName>(), ErrKind::Unauthorized);
            let key = ensure_optional!(
                req.service
                    .state
                    .get_key(
                        &req.address,
                        &req.service.interface,
                        &req.service.external,
                        username,
                    )
                    .await,
                ErrKind::Unauthorized
            );

            req.message
                .integrity(&key)
                .map_err(|e| RouterError::Other(e))?;
            req.auth = Some(Auth { username, key });
        }

        Self::handle(bytes, req)
    }
}

pub struct Operationer<T>
where
    T: Observer + 'static,
{
    service: ServiceContext<T>,
    decoder: Decoder,
    bytes: BytesMut,
}

impl<'a, T> Operationer<T>
where
    T: Observer + 'static,
{
    pub(crate) fn new(service: ServiceContext<T>) -> Self {
        Self {
            bytes: BytesMut::with_capacity(4096),
            decoder: Decoder::new(),
            service,
        }
    }

    pub async fn route<'c, 'b: 'c>(
        &'b mut self,
        bytes: &'b [u8],
        address: SocketAddr,
    ) -> Result<Option<Response<'c>>, StunError> {
        match self.decoder.decode(bytes)? {
            Payload::ChannelData(channel) => {
                unimplemented!()
            }
            Payload::Message(message) => {
                let req = Requet {
                    service: &self.service,
                    message: &message,
                    auth: None,
                    address,
                };

                let res = match message.method {
                    Method::Allocate(Kind::Request) => Allocate::route(&mut self.bytes, req).await,
                    _ => Err(RouterError::Other(StunError::InvalidInput)),
                };

                match res {
                    Err(RouterError::Reject(err)) => {
                        let mut message = MessageWriter::extend(
                            if let Some(it) = message.method.into_error() {
                                it
                            } else {
                                return Ok(None);
                            },
                            &message,
                            &mut self.bytes,
                        );

                        message.append::<ErrorCode>(Error::from(err));
                        message.append::<Realm>(&self.service.realm);
                        message.flush(None)?;

                        Ok(Some(Response {
                            kind: StunClass::Message,
                            bytes: &self.bytes,
                            relay: None,
                        }))
                    }
                    Err(RouterError::Other(err)) => Err(err),
                    Ok(it) => Ok(it),
                }
            }
        }
    }
}
