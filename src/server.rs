use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use bytes::Bytes;
use codec::message::methods::Method;
use dashmap::DashMap;
use service::{
    forwarding::{ForwardResult, Outbound},
    session::Identifier,
};

use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    task::JoinSet,
    time::sleep,
};

use crate::{
    Service,
    config::{Config, Interface, Ssl},
    server::{tcp::TcpServer, udp::UdpServer},
    statistics::{Statistics, Stats},
};

pub const MAX_MESSAGE_SIZE: usize = 4096;

pub async fn start_server(config: Config, service: Service, statistics: Statistics) -> Result<()> {
    let exchanger = Exchanger::default();

    let mut servers = JoinSet::new();

    for interface in config.server.interfaces {
        match interface {
            Interface::Udp {
                listen,
                external,
                idle_timeout,
                mtu,
            } => {
                servers.spawn(UdpServer::start(
                    ListenOptions {
                        transport: Transport::Udp,
                        idle_timeout,
                        ssl: None,
                        external,
                        listen,
                        mtu,
                    },
                    service.clone(),
                    statistics.clone(),
                    exchanger.clone(),
                ));
            }
            Interface::Tcp {
                listen,
                external,
                idle_timeout,
                ssl,
            } => {
                servers.spawn(TcpServer::start(
                    ListenOptions {
                        transport: Transport::Tcp,
                        idle_timeout,
                        external,
                        listen,
                        mtu: 0,
                        ssl,
                    },
                    service.clone(),
                    statistics.clone(),
                    exchanger.clone(),
                ));
            }
        };
    }

    // As soon as one server exits, all servers will be exited to ensure the
    // availability of all servers.
    if let Some(res) = servers.join_next().await {
        servers.abort_all();

        return Ok(res??);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transport {
    Udp,
    Tcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutboundType {
    Message(Method),
    ChannelData,
}

/// Handles packet forwarding between transport protocols.
#[derive(Clone)]
struct Exchanger(Arc<DashMap<SocketAddr, UnboundedSender<(Bytes, OutboundType)>>>);

impl Default for Exchanger {
    fn default() -> Self {
        Self(Arc::new(DashMap::with_capacity(1024)))
    }
}

impl Exchanger {
    /// Get the socket reader for the route.
    ///
    /// Each transport protocol is layered according to its own socket, and
    /// the data forwarded to this socket can be obtained by routing.
    fn get_receiver(&self, interface: SocketAddr) -> UnboundedReceiver<(Bytes, OutboundType)> {
        let (sender, receiver) = unbounded_channel();
        self.0.insert(interface, sender);
        
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
            if let Some(sender) = self.0.get(interface) {
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
        drop(self.0.remove(interface))
    }
}

trait Socket: Send + 'static {
    fn read(&mut self) -> impl Future<Output = Option<Bytes>> + Send;
    fn write(&mut self, buffer: &[u8]) -> impl Future<Output = Result<()>> + Send;
    fn close(&mut self);
}

struct ListenOptions {
    transport: Transport,
    idle_timeout: u32,
    listen: SocketAddr,
    external: SocketAddr,
    ssl: Option<Ssl>,
    mtu: usize,
}

trait Listener: Sized + Send {
    type Socket: Socket;

    fn bind(options: &ListenOptions) -> impl Future<Output = Result<Self>> + Send;
    fn accept(&mut self) -> impl Future<Output = Option<(Self::Socket, SocketAddr)>> + Send;
    fn local_addr(&self) -> Result<SocketAddr>;

    fn start(
        options: ListenOptions,
        service: Service,
        statistics: Statistics,
        exchanger: Exchanger,
    ) -> impl Future<Output = Result<()>> + Send {
        let transport = options.transport;
        let idle_timeout = options.idle_timeout as u64;

        async move {
            let mut listener = Self::bind(&options).await?;
            let local_addr = listener.local_addr()?;

            log::info!(
                "server listening: listen={}, external={}, local addr={local_addr}, transport={transport:?}",
                options.listen,
                options.external,
            );

            while let Some((mut socket, address)) = listener.accept().await {
                let id = Identifier {
                    interface: options.external,
                    source: address,
                };

                let mut receiver = exchanger.get_receiver(address);
                let mut forwarder = service.get_forwarder(address, options.external);
                let reporter = statistics.get_reporter();

                let service = service.clone();
                let exchanger = exchanger.clone();

                tokio::spawn(async move {
                    let sleep = sleep(Duration::from_secs(1));
                    tokio::pin!(sleep);

                    let mut read_delay = 0;

                    loop {
                        tokio::select! {
                            Some(buffer) = socket.read() => {
                                read_delay = 0;

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
                            _ = &mut sleep => {
                                read_delay += 1;

                                if read_delay >= idle_timeout {
                                    break;
                                }
                            }
                            else => {
                                break;
                            }
                        }
                    }

                    // close the socket
                    socket.close();

                    // When the socket connection is closed, the procedure to close the session is
                    // process directly once, avoiding the connection being disconnected
                    // directly without going through the closing
                    // process.
                    service.get_session_manager().refresh(&id, 0);

                    exchanger.remove(&address);

                    log::info!("socket disconnect: addr={address:?}, interface={local_addr:?}, transport={transport:?}");
                });
            }

            log::error!("server shutdown: interface={local_addr:?}, transport={transport:?}");

            Ok(())
        }
    }
}

mod udp {
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

    use crate::server::{ListenOptions, Listener, Socket};

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

        fn close(&mut self) {
            self.bytes_receiver.close();

            let _ = self.close_signal_sender.send(self.addr);
        }
    }

    pub struct UdpServer {
        receiver: UnboundedReceiver<(UdpSocket, SocketAddr)>,
        socket: Arc<TokioUdpSocket>,
    }

    impl Listener for UdpServer {
        type Socket = UdpSocket;

        async fn bind(options: &ListenOptions) -> Result<Self> {
            let socket = Arc::new(TokioUdpSocket::bind(options.listen).await?);
            let (socket_sender, socket_receiver) = unbounded_channel::<(UdpSocket, SocketAddr)>();
            let (close_signal_sender, mut close_signal_receiver) =
                unbounded_channel::<SocketAddr>();

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

                                if let Some(stream) = sockets.get(&addr) {
                                    if stream.try_send(Bytes::copy_from_slice(&buffer[..size])).is_err()
                                    {
                                        sockets.remove(&addr);
                                    }
                                } else {
                                    let (tx, bytes_receiver) = channel::<Bytes>(100);
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
}

mod tcp {
    use std::{io::Error, net::SocketAddr};

    #[cfg(feature = "ssl")]
    use std::sync::Arc;

    use anyhow::Result;
    use bytes::{Bytes, BytesMut};
    use codec::Decoder;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
        net::{TcpListener as TokioTcpListener, TcpStream},
        sync::mpsc::{Sender, UnboundedReceiver, channel, unbounded_channel},
    };

    #[cfg(feature = "ssl")]
    use tokio_rustls::{
        TlsAcceptor,
        rustls::{
            ServerConfig,
            pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
        },
        server::TlsStream,
    };

    use crate::server::{ListenOptions, Listener, MAX_MESSAGE_SIZE, Socket};

    enum MaybeSslStream {
        #[cfg(feature = "ssl")]
        Ssl(TlsStream<TcpStream>),
        Base(TcpStream),
    }

    impl MaybeSslStream {
        fn split(self) -> (Reader, Writer) {
            use tokio::io::split;

            match self {
                Self::Base(it) => {
                    let (rx, tx) = split(it);

                    (Reader::Base(rx), Writer::Base(tx))
                }
                #[cfg(feature = "ssl")]
                Self::Ssl(it) => {
                    let (rx, tx) = split(it);

                    (Reader::Ssl(rx), Writer::Ssl(tx))
                }
            }
        }
    }

    enum Reader {
        #[cfg(feature = "ssl")]
        Ssl(ReadHalf<TlsStream<TcpStream>>),
        Base(ReadHalf<TcpStream>),
    }

    impl Reader {
        async fn read_buf(&mut self, buffer: &mut BytesMut) -> Result<usize, Error> {
            match self {
                Self::Base(it) => it.read_buf(buffer).await,
                #[cfg(feature = "ssl")]
                Self::Ssl(it) => it.read_buf(buffer).await,
            }
        }
    }

    enum Writer {
        #[cfg(feature = "ssl")]
        Ssl(WriteHalf<TlsStream<TcpStream>>),
        Base(WriteHalf<TcpStream>),
    }

    impl Writer {
        async fn write_all(&mut self, buffer: &[u8]) -> Result<(), Error> {
            match self {
                Self::Base(it) => it.write_all(buffer).await,
                #[cfg(feature = "ssl")]
                Self::Ssl(it) => it.write_all(buffer).await,
            }
        }
    }

    pub struct TcpSocket {
        writer: Writer,
        receiver: UnboundedReceiver<Bytes>,
        close_signal_sender: Sender<()>,
    }

    impl TcpSocket {
        fn new(stream: MaybeSslStream, addr: SocketAddr) -> Self {
            let (close_signal_sender, mut close_signal_receiver) = channel::<()>(1);
            let (tx, receiver) = unbounded_channel::<Bytes>();
            let (mut reader, writer) = stream.split();

            tokio::spawn(async move {
                let mut buffer = BytesMut::new();

                'a: loop {
                    tokio::select! {
                        Ok(size) = reader.read_buf(&mut buffer) => {
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
                                    break 'a;
                                }
                            }
                        }
                        _ = close_signal_receiver.recv() => {
                            break;
                        }
                        else => {
                            break;
                        }
                    }
                }
            });

            Self {
                close_signal_sender,
                writer,
                receiver,
            }
        }
    }

    impl Socket for TcpSocket {
        async fn read(&mut self) -> Option<Bytes> {
            self.receiver.recv().await
        }

        async fn write(&mut self, buffer: &[u8]) -> Result<()> {
            Ok(self.writer.write_all(buffer).await?)
        }

        fn close(&mut self) {
            self.receiver.close();

            let _ = self.close_signal_sender.send(());
        }
    }

    pub struct TcpServer {
        socket_receiver: UnboundedReceiver<(TcpSocket, SocketAddr)>,
        local_addr: SocketAddr,
    }

    impl Listener for TcpServer {
        type Socket = TcpSocket;

        async fn bind(options: &ListenOptions) -> Result<Self> {
            #[cfg(feature = "ssl")]
            let acceptor = if let Some(ssl) = &options.ssl {
                Some(TlsAcceptor::from(Arc::new(
                    ServerConfig::builder()
                        .with_no_client_auth()
                        .with_single_cert(
                            CertificateDer::pem_file_iter(ssl.certificate_chain.clone())?
                                .collect::<Result<Vec<_>, _>>()?,
                            PrivateKeyDer::from_pem_file(ssl.private_key.clone())?,
                        )?,
                )))
            } else {
                None
            };

            let listener = TokioTcpListener::bind(options.listen).await?;
            let local_addr = listener.local_addr()?;

            let (tx, socket_receiver) = unbounded_channel::<(TcpSocket, SocketAddr)>();
            tokio::spawn(async move {
                while let Ok((socket, addr)) = listener.accept().await {
                    // Disable the Nagle algorithm.
                    // because to maintain real-time, any received data should be processed
                    // as soon as possible.
                    if let Err(e) = socket.set_nodelay(true) {
                        log::warn!("tls socket set nodelay failed!: addr={addr}, err={e}");
                    }

                    #[cfg(feature = "ssl")]
                    if let Some(acceptor) = acceptor.clone() {
                        let tx = tx.clone();

                        tokio::spawn(async move {
                            if let Ok(socket) = acceptor.accept(socket).await {
                                let _ = tx.send((
                                    TcpSocket::new(MaybeSslStream::Ssl(socket), addr),
                                    addr,
                                ));
                            };
                        });

                        continue;
                    }

                    if tx
                        .send((TcpSocket::new(MaybeSslStream::Base(socket), addr), addr))
                        .is_err()
                    {
                        break;
                    }
                }
            });

            Ok(Self {
                socket_receiver,
                local_addr,
            })
        }

        async fn accept(&mut self) -> Option<(Self::Socket, SocketAddr)> {
            self.socket_receiver.recv().await
        }

        fn local_addr(&self) -> Result<SocketAddr> {
            Ok(self.local_addr)
        }
    }
}
