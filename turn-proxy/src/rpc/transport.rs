use std::io::IoSlice;
use std::{
    net::SocketAddr,
    sync::Arc,
};

use anyhow::{
    Result,
    anyhow,
};

use bytes::{
    BytesMut,
    Bytes,
};

use tokio::sync::Mutex;
use tokio::sync::mpsc::{
    UnboundedSender,
    UnboundedReceiver,
    unbounded_channel,
};

use tokio::{
    net::*,
    io::*,
};

pub struct Protocol;

const PROTOCOL_MAGIC: u8 = 0xAA;

#[derive(Debug)]
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
    pub fn decode_head(buf: &[u8]) -> Result<Option<(usize, u8)>> {
        if buf[0] != PROTOCOL_MAGIC {
            return Err(anyhow!("invalid packet!"));
        }

        let size = u16::from_be_bytes(buf[1..3].try_into()?);
        let size = size as usize;
        Ok(if size <= buf.len() {
            Some((size, buf[4]))
        } else {
            None
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
    pub fn decode(buf: &[u8]) -> Result<Option<ProtocolRecvRef>> {
        let (size, to) = if let Some(ret) = Self::decode_head(buf)? {
            ret
        } else {
            return Ok(None);
        };

        Ok(Some(ProtocolRecvRef {
            data: &buf[4..size],
            size,
            to,
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

        let size_buf = u16::to_be_bytes(input.len() as u16 + 4);
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
    receiver: UnboundedReceiver<(Bytes, u8)>,
    sender: UnboundedSender<(Bytes, u8)>,
}

impl OrderTransport {
    pub async fn new(addr: TransportAddr) -> Result<Self> {
        let listener = TcpListener::bind(addr.bind).await?;
        let (recv_sender, receiver) = unbounded_channel::<(Bytes, u8)>();
        let (sender, send_receiver) = unbounded_channel::<(Bytes, u8)>();
        let send_receiver = Arc::new(Mutex::new(send_receiver));

        tokio::spawn(async move {
            while let Ok((mut socket, source)) = listener.accept().await {
                if source.ip() != addr.proxy.ip() {
                    return;
                }

                let sender = recv_sender.clone();
                let receiver = send_receiver.clone();

                tokio::spawn(async move {
                    let mut buf = BytesMut::new();

                    loop {
                        let mut receiver = receiver.lock().await;
                        tokio::select! {
                            Ok(size) = socket.read_buf(&mut buf) => {
                                if size > 0 {
                                    if let Ok(ret) = Protocol::decode_head(&buf) {
                                        if let Some((size, to)) = ret {
                                            let data = buf.split_to(size).split_off(4);
                                            if sender.send((data.freeze(), to)).is_err() {
                                                break;
                                            }
                                        }
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            },
                            Some((buf, to)) = receiver.recv() => {
                                let head = Protocol::encode_header(&buf, to);
                                let vect = [IoSlice::new(&head), IoSlice::new(&buf)];
                                if socket.write_vectored(&vect).await.is_err() {
                                    break;
                                }
                            },
                            else => {
                                break;
                            }
                        }
                    }
                });
            }
        });

        Ok(Self {
            receiver,
            sender,
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
        self.sender.send((Bytes::copy_from_slice(buf), to))?;
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
    pub async fn recv(&mut self) -> Option<(Bytes, u8)> {
        self.receiver.recv().await
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
        let head = Protocol::encode_header(buf, to);
        self.socket.send_to(&head, self.addr.proxy).await?;
        self.socket.send_to(buf, self.addr.proxy).await?;

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
            return Ok(None);
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
