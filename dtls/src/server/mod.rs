mod session;

use std::io::*;
use std::net::UdpSocket;
use std::collections::HashMap;
use std::net::SocketAddr;

use session::Session;

pub struct Server {
    inner: UdpSocket,
    sessions: HashMap<SocketAddr, Session>
}
