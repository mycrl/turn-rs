use futures::prelude::*;
use std::task::{Context, Poll};
use std::{marker::Unpin, pin::Pin};
use tokio::io::{AsyncRead, AsyncWrite, Error};
use tokio::net::TcpStream;

pub struct Socket {
    stream: TcpStream,
}

impl Socket {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream
        }
    }
}
