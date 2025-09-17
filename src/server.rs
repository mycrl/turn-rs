use std::{io::Error, net::SocketAddr, sync::Arc};

use ahash::{HashMap, HashMapExt};
use anyhow::Result;
use bytes::Bytes;
use codec::message::methods::Method;
use parking_lot::RwLock;
use service::{
    forwarding::{ForwardResult, Outbound},
    session::Identifier,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::{
    Service,
    config::{Config, Interface, Transport},
    server::{tcp::TcpServer, udp::UdpServer},
    statistics::{Statistics, Stats},
};

pub const MAX_MESSAGE_SIZE: usize = 4096;

pub async fn start_server(config: Config, service: Service, statistics: Statistics) -> Result<()> {
    let exchanger = Exchanger::default();

    for interface in &config.turn.interfaces {
        match interface.transport {
            Transport::Udp => {
                UdpServer::start(
                    interface,
                    service.clone(),
                    statistics.clone(),
                    exchanger.clone(),
                )
                .await?;
            }
            Transport::Tcp => {
                TcpServer::start(
                    interface,
                    service.clone(),
                    statistics.clone(),
                    exchanger.clone(),
                )
                .await?;
            }
        };
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutboundType {
    Message(Method),
    ChannelData,
}

/// Handles packet forwarding between transport protocols.
#[derive(Clone)]
struct Exchanger(Arc<RwLock<HashMap<SocketAddr, UnboundedSender<(Bytes, OutboundType)>>>>);

impl Default for Exchanger {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(HashMap::with_capacity(1024))))
    }
}

impl Exchanger {
    /// Get the socket reader for the route.
    ///
    /// Each transport protocol is layered according to its own socket, and
    /// the data forwarded to this socket can be obtained by routing.
    fn get_receiver(&self, interface: SocketAddr) -> UnboundedReceiver<(Bytes, OutboundType)> {
        let (sender, receiver) = unbounded_channel();
        self.0.write().insert(interface, sender);
        receiver
    }

    /// Send data to dispatcher.
    ///
    /// By specifying the socket identifier and destination address, the route
    /// is forwarded to the corresponding socket. However, it should be noted
    /// that calling this function will not notify whether the socket exists.
    /// If it does not exist, the data will be discarded by default.
    fn send(&self, interface: &SocketAddr, ty: OutboundType, data: Bytes) {
        let mut is_destroy = false;

        {
            if let Some(sender) = self.0.read().get(interface) {
                if sender.send((data, ty)).is_err() {
                    is_destroy = true;
                }
            }
        }

        if is_destroy {
            self.remove(interface);
        }
    }

    /// delete socket.
    pub fn remove(&self, interface: &SocketAddr) {
        drop(self.0.write().remove(interface))
    }
}

trait Socket: Send + 'static {
    fn read(&mut self) -> impl Future<Output = Option<Bytes>> + Send;
    fn write(&mut self, buffer: &[u8]) -> impl Future<Output = Result<(), Error>> + Send;
}

trait Listener: Sized + Send {
    type Socket: Socket;

    fn bind(interface: &Interface) -> impl Future<Output = Result<Self, Error>> + Send;
    fn accept(&mut self) -> impl Future<Output = Option<(Self::Socket, SocketAddr)>> + Send;
    fn local_addr(&self) -> Result<SocketAddr, Error>;

    fn start(
        interface: &Interface,
        service: Service,
        statistics: Statistics,
        exchanger: Exchanger,
    ) -> impl Future<Output = Result<()>> + Send {
        let transport = interface.transport;

        async move {
            let mut listener = Self::bind(interface).await?;
            let local_addr = listener.local_addr()?;

            log::info!(
                "server listening: listen={}, external={}, local addr={local_addr}, transport={transport:?}",
                interface.listen,
                interface.external,
            );

            while let Some((mut socket, address)) = listener.accept().await {
                let id = Identifier {
                    interface: interface.external,
                    source: address,
                };

                let mut receiver = exchanger.get_receiver(address);
                let mut forwarder = service.get_forwarder(address, interface.external);
                let reporter = statistics.get_reporter(transport);

                let service = service.clone();
                let exchanger = exchanger.clone();

                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            Some(buffer) = socket.read() => {
                                if let ForwardResult::Outbound(outbound) = forwarder.forward(&buffer, address).await
                                {
                                    let (ty, bytes, target) = match outbound {
                                        Outbound::Message {
                                            method,
                                            bytes,
                                            target,
                                        } => (OutboundType::Message(method), bytes, target),
                                        Outbound::ChannelData { bytes, target } => {
                                            (OutboundType::ChannelData, bytes, target)
                                        }
                                    };

                                    if let Some(endpoint) = target.endpoint {
                                        exchanger.send(&endpoint, ty, Bytes::copy_from_slice(bytes));
                                    } else {
                                        if socket.write(bytes).await.is_err() {
                                            break;
                                        }

                                        reporter.send(
                                            &id,
                                            &[Stats::SendBytes(bytes.len()), Stats::SendPkts(1)],
                                        );

                                        if let OutboundType::Message(method) = ty {
                                            if method.is_error() {
                                                reporter.send(&id, &[Stats::ErrorPkts(1)]);
                                            }
                                        }
                                    }
                                } else {
                                    break;
                                }
                            }
                            Some((bytes, method)) = receiver.recv() => {
                                if socket.write(&bytes).await.is_err() {
                                    break;
                                } else {
                                    reporter.send(&id, &[Stats::SendBytes(bytes.len()), Stats::SendPkts(1)]);
                                }

                                // The channel data needs to be aligned in multiples of 4 in
                                // tcp. If the channel data is forwarded to tcp, the alignment
                                // bit needs to be filled, because if the channel data comes
                                // from udp, it is not guaranteed to be aligned and needs to be
                                // checked.
                                if transport == Transport::Tcp {
                                    if method == OutboundType::ChannelData {
                                        let pad = bytes.len() % 4;
                                        if pad > 0 && socket.write(&[0u8; 8][..(4 - pad)]).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                            else => {
                                break;
                            }
                        }
                    }

                    // When the socket connection is closed, the procedure to close the session is
                    // process directly once, avoiding the connection being disconnected
                    // directly without going through the closing
                    // process.
                    service.get_session_manager().refresh(&id, 0);

                    exchanger.remove(&address);

                    log::info!("socket disconnect: addr={address:?}, interface={local_addr:?}");
                });
            }

            log::error!("server shutdown: interface={local_addr:?}, transport={transport:?}");

            Ok(())
        }
    }
}

mod udp {
    use std::{
        io::{Error, ErrorKind},
        net::SocketAddr,
        sync::Arc,
    };

    use ahash::{HashMap, HashMapExt};
    use bytes::{Bytes, BytesMut};
    use tokio::{
        net::UdpSocket as TokioUdpSocket,
        sync::mpsc::{Receiver, Sender, UnboundedReceiver, channel, unbounded_channel},
    };

    use crate::{
        config::Interface,
        server::{Listener, Socket},
    };

    pub struct UdpSocket {
        bytes_receiver: Receiver<Bytes>,
        socket: Arc<TokioUdpSocket>,
        addr: SocketAddr,
    }

    impl Socket for UdpSocket {
        async fn read(&mut self) -> Option<Bytes> {
            self.bytes_receiver.recv().await
        }

        async fn write(&mut self, buffer: &[u8]) -> Result<(), Error> {
            if let Err(e) = self.socket.send_to(buffer, self.addr).await {
                // Note: An error will also be reported when the remote host is
                // shut down, which is not processed yet, but a
                // warning will be issued.
                if e.kind() != ErrorKind::ConnectionReset {
                    return Err(e);
                }
            }

            Ok(())
        }
    }

    pub struct UdpServer {
        receiver: UnboundedReceiver<(UdpSocket, SocketAddr)>,
        socket: Arc<TokioUdpSocket>,
    }

    impl Listener for UdpServer {
        type Socket = UdpSocket;

        async fn bind(interface: &Interface) -> Result<Self, Error> {
            let socket = Arc::new(TokioUdpSocket::bind(interface.listen).await?);
            let (socket_sender, socket_receiver) = unbounded_channel::<(UdpSocket, SocketAddr)>();

            {
                let socket = socket.clone();

                let mut buffer = BytesMut::zeroed(interface.mtu);

                tokio::spawn(async move {
                    let mut sockets = HashMap::<SocketAddr, Sender<Bytes>>::with_capacity(1024);

                    loop {
                        let (size, addr) = match socket.recv_from(&mut buffer).await {
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

                        if let Some(stream) = sockets.get(&addr) {
                            if let Err(e) = stream.try_send(Bytes::copy_from_slice(&buffer[..size]))
                            {
                                sockets.remove(&addr);

                                log::info!("udp stream error={e}: addr={addr}");
                            }
                        } else {
                            let (tx, bytes_receiver) = channel::<Bytes>(100);
                            sockets.insert(addr, tx);

                            if socket_sender
                                .send((
                                    UdpSocket {
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

                            log::info!("udp server new connection: addr={addr}");
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

        fn local_addr(&self) -> Result<SocketAddr, Error> {
            self.socket.local_addr()
        }
    }
}

mod tcp {
    use std::{io::Error, net::SocketAddr};

    use bytes::{Bytes, BytesMut};
    use codec::Decoder;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener as TokioTcpListener, TcpStream, tcp::OwnedWriteHalf},
        sync::mpsc::{UnboundedReceiver, unbounded_channel},
    };

    use crate::{
        config::Interface,
        server::{Listener, MAX_MESSAGE_SIZE, Socket},
    };

    pub struct TcpSocket {
        writer: OwnedWriteHalf,
        receiver: UnboundedReceiver<Bytes>,
    }

    impl TcpSocket {
        fn new(stream: TcpStream, addr: SocketAddr) -> Self {
            // Disable the Nagle algorithm.
            // because to maintain real-time, any received data should be processed
            // as soon as possible.
            let _ = stream.set_nodelay(true);

            let (tx, receiver) = unbounded_channel::<Bytes>();
            let (mut reader, writer) = stream.into_split();

            tokio::spawn(async move {
                let mut buffer = BytesMut::new();

                'a: while let Ok(size) = reader.read_buf(&mut buffer).await {
                    if size == 0 {
                        break;
                    }

                    // The minimum length of a stun message will not be less
                    // than 4.
                    if buffer.len() < 4 {
                        continue;
                    }

                    // Limit the maximum length of messages to 2048, this is to prevent buffer
                    // overflow attacks.
                    if buffer.len() > MAX_MESSAGE_SIZE * 3 {
                        break;
                    }

                    loop {
                        if buffer.len() <= 4 {
                            break;
                        }

                        // Try to get the message length, if the currently
                        // received data is less than the message length, jump
                        // out of the current loop and continue to receive more
                        // data.
                        let size = match Decoder::message_size(&buffer, true) {
                            Err(_) => break,
                            Ok(size) => {
                                if size > MAX_MESSAGE_SIZE {
                                    log::warn!(
                                        "tcp message size too large: \
                                            size={size}, \
                                            max={MAX_MESSAGE_SIZE}, \
                                            addr={addr:?}"
                                    );

                                    break 'a;
                                }

                                if size > buffer.len() {
                                    break;
                                }

                                size
                            }
                        };

                        if tx.send(buffer.split_to(size).freeze()).is_err() {
                            break;
                        }
                    }
                }
            });

            Self { writer, receiver }
        }
    }

    impl Socket for TcpSocket {
        async fn read(&mut self) -> Option<Bytes> {
            self.receiver.recv().await
        }

        async fn write(&mut self, buffer: &[u8]) -> Result<(), Error> {
            self.writer.write_all(buffer).await
        }
    }

    pub struct TcpServer(TokioTcpListener);

    impl Listener for TcpServer {
        type Socket = TcpSocket;

        async fn bind(interface: &Interface) -> Result<Self, Error> {
            Ok(Self(TokioTcpListener::bind(interface.listen).await?))
        }

        async fn accept(&mut self) -> Option<(Self::Socket, SocketAddr)> {
            self.0
                .accept()
                .await
                .ok()
                .map(|(socket, addr)| (TcpSocket::new(socket, addr), addr))
        }

        fn local_addr(&self) -> Result<SocketAddr, Error> {
            self.0.local_addr()
        }
    }
}
