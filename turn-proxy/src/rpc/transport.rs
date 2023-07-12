use std::net::SocketAddr;

use anyhow::{
    Result,
    anyhow,
};

use tokio::{
    net::*,
    io::*,
};

pub struct Protocol;

const PROTOCOL_MAGIC: u8 = 0xAA;

pub struct ProtocolRecvRef<'a> {
    pub size: usize,
    pub to: u8,
    pub data: &'a [u8],
}

impl Protocol {
    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub fn decode(buf: &[u8]) -> Result<Option<ProtocolRecvRef>> {
        if buf[0] != PROTOCOL_MAGIC {
            return Err(anyhow!("invalid packet!"));
        }

        let size = u16::from_be_bytes(buf[1..3].try_into()?);
        let size = size as usize;
        if size > buf.len() {
            return Ok(None);
        }

        Ok(Some(ProtocolRecvRef {
            data: &buf[4..size],
            to: buf[4],
            size,
        }))
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub fn encode_header(input: &[u8], to: u8) -> [u8; 4] {
        let mut dst = [0u8; 4];
        dst[0] = PROTOCOL_MAGIC;

        let size_buf = u16::to_be_bytes(input.len() as u16);
        dst[1] = size_buf[0];
        dst[2] = size_buf[1];

        dst[3] = to;
        dst
    }
}

pub struct OrderTransport {
    socket: TcpStream,
    buf: [u8; 2048],
}

impl OrderTransport {
    pub async fn new(addr: SocketAddr) -> Result<Self> {
        Ok(Self {
            socket: TcpSocket::new_v4()?.connect(addr).await?,
            buf: [0u8; 2048],
        })
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub async fn send(&mut self, buf: &[u8], to: u8) -> Result<()> {
        let head = Protocol::encode_header(buf, to);
        self.socket.write_all(&head).await?;
        self.socket.write_all(buf).await?;
        self.socket.flush().await?;
        Ok(())
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub async fn recv(&mut self) -> Result<Option<(&[u8], u8)>> {
        let size = self.socket.read(&mut self.buf).await?;
        if let Some(ret) = Protocol::decode(&self.buf[..size])? {
            Ok(Some((ret.data, ret.to)))
        } else {
            Ok(None)
        }
    }
}

pub struct Transport {
    remote_addr: SocketAddr,
    socket: UdpSocket,
    buf: [u8; 2048],
}

impl Transport {
    pub async fn new(addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(addr).await?;
        Ok(Self {
            buf: [0u8; 2048],
            remote_addr: addr,
            socket,
        })
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub async fn send(&self, buf: &[u8], to: u8) -> Result<()> {
        let head = Protocol::encode_header(buf, to);
        self.socket.send_to(&head, self.remote_addr).await?;
        self.socket.send_to(buf, self.remote_addr).await?;
        Ok(())
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub async fn recv(&mut self) -> Result<Option<(&[u8], u8)>> {
        let (size, source) = self.socket.recv_from(&mut self.buf).await?;
        if source != self.remote_addr {
            return Ok(None);
        }

        if let Some(ret) = Protocol::decode(&self.buf[..size])? {
            Ok(Some((ret.data, ret.to)))
        } else {
            Ok(None)
        }
    }
}
