use std::net::SocketAddr;

use bytes::{Bytes, BytesMut};
use codec::message::{Message, MessageEncoder, attributes::ErrorKind, methods::BINDING_REQUEST};
use rand::Rng;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Codec(codec::Error),
    MissingAttribute(&'static str),
    Stun(ErrorKind),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<codec::Error> for Error {
    fn from(value: codec::Error) -> Self {
        Self::Codec(value)
    }
}

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Self::Stun(value)
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub trait RequestStream {
    fn read<'a>(&'a mut self) -> Result<Message<'a>, std::io::Error>;
    fn send(&mut self, bytes: Bytes) -> Result<(), std::io::Error>;
}

pub trait RequestBuilder {
    type Response;

    fn request<T>(&self, stream: &mut T) -> impl Future<Output = Result<Self::Response, Error>>
    where
        T: RequestStream;
}

pub struct Request<T> {
    payload: T,
}

pub struct Binding;

pub struct BindingResponse {
    pub xor_mapped_address: SocketAddr,
    pub mapped_address: SocketAddr,
    pub response_origin: SocketAddr,
}

impl RequestBuilder for Request<Binding> {
    type Response = BindingResponse;

    async fn request<T>(&self, stream: &mut T) -> Result<Self::Response, Error>
    where
        T: RequestStream,
    {
        let mut token = [0u8; 12];
        rand::rng().fill(&mut token);

        let mut bytes = BytesMut::with_capacity(1500);

        {
            MessageEncoder::new(BINDING_REQUEST, &token, &mut bytes).flush(None)?;
        }

        stream.send(bytes.freeze())?;

        let message = stream.read()?;

        todo!()
    }
}
