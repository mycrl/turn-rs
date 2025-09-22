use std::net::SocketAddr;

use codec::message::methods::Method;

/// The target of the response.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResponseTarget {
    pub endpoint: Option<SocketAddr>,
    pub relay: Option<SocketAddr>,
}

/// The response.
#[derive(Debug)]
pub struct Response<'a> {
    method: Option<Method>,
    payload: &'a [u8],
    target: ResponseTarget,
}

impl<'a> Response<'a> {
    /// The type of the response.
    #[inline(always)]
    pub fn is_channel_data(&self) -> bool {
        self.method.is_none()
    }

    /// The error of the response.
    #[inline(always)]
    pub fn is_error(&self) -> bool {
        self.method.map(|it| it.is_error()).unwrap_or(false)
    }

    /// The payload of the response.
    #[inline(always)]
    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }

    /// The target of the response.
    #[inline(always)]
    pub fn target(&self) -> &ResponseTarget {
        &self.target
    }
}

pub(crate) struct ResponseBuilder<'a>(Response<'a>);

impl<'a> ResponseBuilder<'a> {
    /// Create a new message response builder.
    #[inline(always)]
    pub fn message(method: Method) -> Self {
        Self(Response {
            target: ResponseTarget::default(),
            method: Some(method),
            payload: &[],
        })
    }

    /// Create a new channel data response builder.
    #[inline(always)]
    pub fn channel_data() -> Self {
        Self(Response {
            target: ResponseTarget::default(),
            method: None,
            payload: &[],
        })
    }

    /// Set the payload of the response.
    #[inline(always)]
    pub fn payload(mut self, payload: &'a [u8]) -> Self {
        self.0.payload = payload;
        self
    }

    /// Set the target of the response.
    #[inline(always)]
    pub fn target(mut self, target: ResponseTarget) -> Self {
        self.0.target = target;
        self
    }

    /// Build the response.
    #[inline(always)]
    pub fn build(self) -> Response<'a> {
        self.0
    }
}
