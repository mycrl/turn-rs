use std::{io::ErrorKind, net::SocketAddr, sync::Arc};

use ahash::{HashMap, HashMapExt};
use anyhow::Result;
use bytes::{Bytes, BytesMut};
use tokio::{
    net::UdpSocket as TokioUdpSocket,
    sync::mpsc::{
        Receiver, Sender, UnboundedReceiver, UnboundedSender, channel, unbounded_channel,
    },
};

use crate::server::transport::{Server, ServerOptions, Socket};

pub struct UdpSocket {
    close_signal_sender: UnboundedSender<SocketAddr>,
    bytes_receiver: Receiver<Bytes>,
    socket: Arc<TokioUdpSocket>,
    addr: SocketAddr,
}

impl Socket for UdpSocket {
    async fn read(&mut self) -> Option<Bytes> {
        self.bytes_receiver.recv().await
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
    receiver: UnboundedReceiver<(UdpSocket, SocketAddr)>,
    socket: Arc<TokioUdpSocket>,
}

impl Server for UdpServer {
    type Socket = UdpSocket;

    async fn bind(options: &ServerOptions) -> Result<Self> {
        let socket = Arc::new(TokioUdpSocket::bind(options.listen).await?);
        let (socket_sender, socket_receiver) = unbounded_channel::<(UdpSocket, SocketAddr)>();
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
                                    .send((
                                        UdpSocket {
                                            close_signal_sender: close_signal_sender.clone(),
                                            socket: socket.clone(),
                                            bytes_receiver,
                                            addr,
                                        },
                                        addr,
                                    ))
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

    async fn accept(&mut self) -> Option<(UdpSocket, SocketAddr)> {
        self.receiver.recv().await
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }
}
