pub mod request;
pub mod response;
pub mod router;

use crate::{
    Service, ServiceHandler,
    routing::{request::Request, response::Response, router::*},
    session::{Identifier, SessionManager},
};

use std::{net::SocketAddr, sync::Arc};

use bytes::BytesMut;
use codec::{DecodeResult, Decoder, message::methods::*};

pub(crate) struct State<T>
where
    T: ServiceHandler,
{
    pub realm: String,
    pub software: String,
    pub manager: Arc<SessionManager<T>>,
    pub endpoint: SocketAddr,
    pub interface: SocketAddr,
    pub interfaces: Arc<Vec<SocketAddr>>,
    pub handler: T,
}

#[derive(Debug)]
pub enum RouteResult<'a> {
    Exceptional(codec::Error),
    Response(Response<'a>),
    None,
}

pub struct Router<T>
where
    T: ServiceHandler,
{
    id: Identifier,
    state: State<T>,
    decoder: Decoder,
    bytes: BytesMut,
}

impl<T> Router<T>
where
    T: ServiceHandler + Clone,
{
    pub fn new(service: &Service<T>, endpoint: SocketAddr, interface: SocketAddr) -> Self {
        Self {
            bytes: BytesMut::with_capacity(4096),
            decoder: Decoder::default(),
            id: Identifier {
                source: "0.0.0.0:0".parse().unwrap(),
                interface,
            },
            state: State {
                interfaces: service.interfaces.clone(),
                software: service.software.clone(),
                handler: service.handler.clone(),
                manager: service.manager.clone(),
                realm: service.realm.clone(),
                interface,
                endpoint,
            },
        }
    }

    pub async fn route<'a, 'b: 'a>(
        &'b mut self,
        bytes: &'b [u8],
        address: SocketAddr,
    ) -> RouteResult<'a> {
        {
            self.id.source = address;
        }

        (match self.decoder.decode(bytes) {
            Ok(DecodeResult::ChannelData(channel)) => channel_data(
                bytes,
                Request {
                    id: &self.id,
                    state: &self.state,
                    encode_buffer: &mut self.bytes,
                    payload: &channel,
                },
            ),
            Ok(DecodeResult::Message(message)) => {
                let req = Request {
                    id: &self.id,
                    state: &self.state,
                    encode_buffer: &mut self.bytes,
                    payload: &message,
                };

                match req.payload.method() {
                    BINDING_REQUEST => binding(req),
                    ALLOCATE_REQUEST => allocate(req).await,
                    CREATE_PERMISSION_REQUEST => create_permission(req).await,
                    CHANNEL_BIND_REQUEST => channel_bind(req).await,
                    REFRESH_REQUEST => refresh(req).await,
                    SEND_INDICATION => indication(req),
                    _ => None,
                }
            }
            Err(e) => {
                return RouteResult::Exceptional(e);
            }
        })
        .map(RouteResult::Response)
        .unwrap_or(RouteResult::None)
    }
}
