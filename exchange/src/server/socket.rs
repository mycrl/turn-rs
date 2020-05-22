use bytes::BytesMut;
use futures::prelude::*;
use std::task::{Context, Poll};
use std::{marker::Unpin, pin::Pin};
use tokio::io::{AsyncRead, AsyncWrite, Error};
use tokio::net::TcpStream;

pub struct Socket {
    socket: TcpStream
}

impl Socket {
    
}
