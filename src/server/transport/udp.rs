use std::{io::ErrorKind, net::SocketAddr, sync::Arc, task::Poll};

use ahash::{HashMap, HashMapExt};
use anyhow::{Result, anyhow};
use bytes::{Bytes, BytesMut};
use tokio::{
    net::UdpSocket,
    sync::mpsc::{
        Receiver, Sender, UnboundedReceiver, UnboundedSender, channel, unbounded_channel,
    },
};

use crate::server::transport::{Server, ServerOptions, Socket};

pub struct UdpSession {
    close_signal_sender: UnboundedSender<SocketAddr>,
    bytes_receiver: Receiver<Bytes>,
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
}

impl Socket for UdpSession {
    async fn read(&mut self) -> Result<Bytes> {
        self.bytes_receiver
            .recv()
            .await
            .ok_or_else(|| anyhow!("channel closed"))
    }

    async fn write(&mut self, buffer: &[u8]) -> Result<()> {
        if let Err(e) = self.socket.send_to(buffer, self.addr).await {
            // Note: An error will also be reported when the remote host is
            // shut down, which is not processed yet, but a
            // warning will be issued.
            if e.kind() != ErrorKind::ConnectionReset {
                return Err(e.into());
            }
        }

        Ok(())
    }

    async fn close(&mut self) {
        self.bytes_receiver.close();

        let _ = self.close_signal_sender.send(self.addr);
    }
}

pub struct UdpServer {
    receiver: UnboundedReceiver<UdpSession>,
    socket: Arc<UdpSocket>,
}

impl Server for UdpServer {
    type Socket = UdpSession;

    async fn bind(options: &ServerOptions) -> Result<Self> {
        let socket = Arc::new(UdpSocket::bind(options.listen).await?);
        let (socket_sender, socket_receiver) = unbounded_channel::<UdpSession>();
        let (close_signal_sender, mut close_signal_receiver) = unbounded_channel::<SocketAddr>();

        {
            let socket = socket.clone();

            let mut buffer = BytesMut::zeroed(options.mtu);

            tokio::spawn(async move {
                let mut sockets = HashMap::<SocketAddr, Sender<Bytes>>::with_capacity(1024);

                loop {
                    tokio::select! {
                        ret = socket.recv_from(&mut buffer) => {
                            let (size, addr) = match ret {
                                Ok(it) => it,
                                // Note: An error will also be reported when the remote host is
                                // shut down, which is not processed yet, but a
                                // warning will be issued.
                                Err(e) => {
                                    if e.kind() != ErrorKind::ConnectionReset {
                                        log::error!("udp server recv_from error={e}");

                                        break;
                                    } else {
                                        continue;
                                    }
                                }
                            };

                            if size < 4 {
                                continue;
                            }

                            if let Some(stream) = sockets.get(&addr) {
                                if stream.try_send(Bytes::copy_from_slice(&buffer[..size])).is_err()
                                {
                                    sockets.remove(&addr);
                                }
                            } else {
                                let (tx, bytes_receiver) = channel::<Bytes>(100);

                                // Send the first packet to the new socket
                                if tx.try_send(Bytes::copy_from_slice(&buffer[..size])).is_err() {
                                    continue;
                                }

                                sockets.insert(addr, tx);

                                if socket_sender
                                    .send(UdpSession {
                                        close_signal_sender: close_signal_sender.clone(),
                                        socket: socket.clone(),
                                        bytes_receiver,
                                        addr,
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                        Some(addr) = close_signal_receiver.recv() => {
                            let _ = sockets.remove(&addr);
                        }
                        else => {
                            break;
                        }
                    }
                }
            });
        }

        Ok(Self {
            receiver: socket_receiver,
            socket,
        })
    }

    async fn accept(&mut self) -> Result<Poll<(UdpSession, SocketAddr)>> {
        let socket = self
            .receiver
            .recv()
            .await
            .ok_or_else(|| anyhow!("channel closed"))?;

        let addr = socket.addr;

        Ok(Poll::Ready((socket, addr)))
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }
}
