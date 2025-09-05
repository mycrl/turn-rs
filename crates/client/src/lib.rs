pub mod request;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub use codec::message::attributes::Transport;

/// The default bind address is unspecified and port 0.
pub const DEFAULT_BIND: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

/// The default address is localhost and port 3478.
pub const DEFAULT_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3478);

/// SSL configuration
#[derive(Debug, Clone)]
pub struct Ssl {
    ///
    /// SSL private key file
    ///
    pub private_key: String,
    ///
    /// SSL certificate chain file
    ///
    pub certificate_chain: String,
}

pub struct TurnClientBuilder {
    transport: Transport,
    bind: SocketAddr,
    address: SocketAddr,
    ssl: Option<Ssl>,
}

impl Default for TurnClientBuilder {
    fn default() -> Self {
        Self {
            transport: Transport::UDP,
            address: DEFAULT_ADDRESS,
            bind: DEFAULT_BIND,
            ssl: None,
        }
    }
}

impl TurnClientBuilder {
    pub fn with_transport(&mut self, transport: Transport) -> &mut Self {
        self.transport = transport;
        self
    }

    pub fn with_bind(&mut self, bind: SocketAddr) -> &mut Self {
        self.bind = bind;
        self
    }

    pub fn with_address(&mut self, address: SocketAddr) -> &mut Self {
        self.address = address;
        self
    }

    pub fn with_ssl(&mut self, ssl: Ssl) -> &mut Self {
        self.ssl = Some(ssl);
        self
    }

    pub fn build(self) -> TurnClient {
        TurnClient::new(self)
    }
}

pub struct TurnClient {}

impl TurnClient {
    fn new(builder: TurnClientBuilder) -> Self {
        Self {}
    }
}
