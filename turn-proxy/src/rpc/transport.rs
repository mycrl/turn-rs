use std::{
    net::SocketAddr,
    sync::Arc,
};

use anyhow::{
    Result,
    anyhow,
};

use tokio::sync::Mutex;
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

#[derive(Copy, Clone)]
pub struct TransportAddr {
    pub bind: SocketAddr,
    pub proxy: SocketAddr,
}

pub struct OrderTransport {
    socket: Arc<Mutex<Option<TcpStream>>>,
    buf: [u8; 2048],
}

impl OrderTransport {
    pub async fn new(addr: TransportAddr) -> Result<Self> {
        let socket: Arc<Mutex<Option<TcpStream>>> = Default::default();

        let listener = TcpListener::bind(addr.bind).await?;
        let socket_ = socket.clone();
        tokio::spawn(async move {
            while let Ok((socket, source)) = listener.accept().await {
                if source == addr.proxy {
                    let _ = socket_.lock().await.insert(socket);
                }
            }

            Ok::<(), anyhow::Error>(())
        });

        Ok(Self {
            buf: [0u8; 2048],
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
    pub async fn send(&self, buf: &[u8], to: u8) -> Result<bool> {
        Ok(if let Some(socket) = self.socket.lock().await.as_mut() {
            let head = Protocol::encode_header(buf, to);
            socket.write_all(&head).await?;
            socket.write_all(buf).await?;
            socket.flush().await?;

            true
        } else {
            false
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
    pub async fn recv(&mut self) -> Result<Option<(&[u8], u8)>> {
        Ok(if let Some(socket) = self.socket.lock().await.as_mut() {
            let size = socket.read(&mut self.buf).await?;
            if size == 0 {
                return Err(anyhow!("socket read ret size == 0"));
            }

            Protocol::decode(&self.buf[..size])?.map(|ret| (ret.data, ret.to))
        } else {
            None
        })
    }
}

pub struct Transport {
    addr: TransportAddr,
    socket: UdpSocket,
    buf: [u8; 2048],
}

impl Transport {
    pub async fn new(addr: TransportAddr) -> Result<Self> {
        Ok(Self {
            socket: UdpSocket::bind(addr.bind).await?,
            buf: [0u8; 2048],
            addr,
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
        let local_addr = self.socket.local_addr()?;
        let head = Protocol::encode_header(buf, to);
        self.socket.send_to(&head, local_addr).await?;
        self.socket.send_to(buf, local_addr).await?;
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
        if source != self.addr.proxy {
            return Ok(None)
        }

        if size == 0 {
            return Err(anyhow!("socket read ret size == 0"));
        }

        if let Some(ret) = Protocol::decode(&self.buf[..size])? {
            Ok(Some((ret.data, ret.to)))
        } else {
            Ok(None)
        }
    }
}
