use std::net::SocketAddr;

pub struct Node {
    pub host: SocketAddr,
    pub external: SocketAddr,
}
