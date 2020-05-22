use futures::prelude::*;
use tokio::net::TcpStream;

/// TcpSocket
pub struct Sockst {
    socket: TcpStream
}
