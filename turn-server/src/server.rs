use crate::{
    config::{Config, Interface},
    router::Router,
    statistics::Statistics,
};

use std::net::SocketAddr;

use turn::{Observer, Service};

struct ServerStartOptions<T> {
    bind: SocketAddr,
    external: SocketAddr,
    service: Service<T>,
    router: Router,
    statistics: Statistics,
}

trait Server {
    async fn start<T>(options: ServerStartOptions<T>) -> Result<(), anyhow::Error>
    where
        T: Clone + Observer + 'static;
}

mod udp {
    use super::{Server as ServerExt, ServerStartOptions};
    use crate::statistics::Stats;

    use std::{io::ErrorKind::ConnectionReset, ops::Deref, sync::Arc};

    use once_cell::sync::Lazy;
    use stun::Transport;
    use tokio::net::UdpSocket;
    use turn::{Observer, ResponseMethod, SessionAddr};

    static NUM_CPUS: Lazy<usize> = Lazy::new(|| num_cpus::get());

    /// udp socket process thread.
    ///
    /// read the data packet from the UDP socket and hand
    /// it to the proto for processing, and send the processed
    /// data packet to the specified address.
    pub struct Server;

    impl ServerExt for Server {
        async fn start<T>(
            ServerStartOptions {
                bind,
                external,
                service,
                router,
                statistics,
            }: ServerStartOptions<T>,
        ) -> Result<(), anyhow::Error>
        where
            T: Clone + Observer + 'static,
        {
            let socket = Arc::new(UdpSocket::bind(bind).await?);
            let local_addr = socket.local_addr()?;

            tokio::spawn(async move {
                for _ in 0..*NUM_CPUS.deref() {
                    let socket = socket.clone();
                    let router = router.clone();
                    let reporter = statistics.get_reporter(Transport::UDP);
                    let mut operationer = service.get_operationer(external, external);

                    let mut session_addr = SessionAddr {
                        address: external,
                        interface: external,
                    };

                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 2048];

                        loop {
                            // Note: An error will also be reported when the remote host is
                            // shut down, which is not processed yet, but a
                            // warning will be issued.
                            let (size, addr) = match socket.recv_from(&mut buf).await {
                                Err(e) if e.kind() != ConnectionReset => break,
                                Ok(s) => s,
                                _ => continue,
                            };

                            session_addr.address = addr;

                            reporter.send(
                                &session_addr,
                                &[Stats::ReceivedBytes(size as u32), Stats::ReceivedPkts(1)],
                            );

                            // The stun message requires at least 4 bytes. (currently the
                            // smallest stun message is channel data,
                            // excluding content)
                            if size >= 4 {
                                if let Ok(Some(res)) = operationer.route(&buf[..size], addr).await {
                                    let target = res.relay.as_ref().unwrap_or(&addr);
                                    if let Some(ref endpoint) = res.endpoint {
                                        router.send(endpoint, res.method, target, res.bytes);
                                    } else {
                                        if let Err(e) = socket.send_to(res.bytes, target).await {
                                            if e.kind() != ConnectionReset {
                                                break;
                                            }
                                        }

                                        reporter.send(
                                            &session_addr,
                                            &[Stats::SendBytes(res.bytes.len() as u32), Stats::SendPkts(1)],
                                        );

                                        if let ResponseMethod::Stun(method) = res.method {
                                            if method.is_error() {
                                                reporter.send(&session_addr, &[Stats::ErrorPkts(1)]);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
                }

                {
                    let mut session_addr = SessionAddr {
                        address: external,
                        interface: external,
                    };

                    let reporter = statistics.get_reporter(Transport::UDP);
                    let mut receiver = router.get_receiver(external);
                    while let Some((bytes, _, addr)) = receiver.recv().await {
                        session_addr.address = addr;

                        if let Err(e) = socket.send_to(&bytes, addr).await {
                            if e.kind() != ConnectionReset {
                                break;
                            }
                        } else {
                            reporter.send(
                                &session_addr,
                                &[Stats::SendBytes(bytes.len() as u32), Stats::SendPkts(1)],
                            );
                        }
                    }

                    router.remove(&external);
                }

                log::error!("udp server close: interface={:?}", local_addr);
            });

            Ok(())
        }
    }
}

mod tcp {
    use super::{Server as ServerExt, ServerStartOptions};
    use crate::statistics::Stats;

    use std::{
        ops::{Deref, DerefMut},
        sync::Arc,
    };

    use stun::{Decoder, Transport};
    use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpListener, sync::Mutex};
    use turn::{Observer, ResponseMethod, SessionAddr};

    static ZERO_BYTES: [u8; 8] = [0u8; 8];

    /// An emulated double buffer queue, this is used when reading data over
    /// TCP.
    ///
    /// When reading data over TCP, you need to keep adding to the buffer until
    /// you find the delimited position. But this double buffer queue solves
    /// this problem well, in the queue, the separation is treated as the first
    /// read operation and after the separation the buffer is reversed and
    /// another free buffer is used for writing the data.
    ///
    /// If the current buffer in the separation after the existence of
    /// unconsumed data, this time the unconsumed data will be copied to another
    /// free buffer, and fill the length of the free buffer data, this time to
    /// write data again when you can continue to fill to the end of the
    /// unconsumed data.
    ///
    /// This queue only needs to copy the unconsumed data without duplicating
    /// the memory allocation, which will reduce a lot of overhead.
    struct DoubleBufferQueue {
        buffers: [(Vec<u8>, usize /* len */); 2],
        index: usize,
    }

    impl Default for DoubleBufferQueue {
        #[rustfmt::skip]
        fn default() -> Self {
            Self {
                index: 0,
                buffers: [
                    (vec![0u8; 2048], 0), 
                    (vec![0u8; 2048], 0),
                ],
            }
        }
    }

    impl Deref for DoubleBufferQueue {
        type Target = [u8];

        fn deref(&self) -> &Self::Target {
            &self.buffers[self.index].0[..]
        }
    }

    impl DerefMut for DoubleBufferQueue {
        // Writes need to take into account overwriting written data, so fetching the
        // writable buffer starts with the internal cursor.
        fn deref_mut(&mut self) -> &mut Self::Target {
            let len = self.buffers[self.index].1;
            &mut self.buffers[self.index].0[len..]
        }
    }

    impl DoubleBufferQueue {
        fn len(&self) -> usize {
            self.buffers[self.index].1
        }

        /// The buffer does not automatically advance the cursor as BytesMut
        /// does, and you need to manually advance the length of the data
        /// written.
        fn advance(&mut self, len: usize) {
            self.buffers[self.index].1 += len;
        }

        #[rustfmt::skip]
        fn split(&mut self, len: usize) -> &[u8] {
            let (ref buffer, size) = self.buffers[self.index];

            // The length of the separation cannot be greater than the length of the data.
            assert!(len <= size);

            // Length of unconsumed data
            let diff_size = size - len;
            {
                // The current buffer is no longer in use, resetting the content length.
                self.buffers[self.index].1 = 0;

                // Invert the buffer.
                self.index = if self.index == 0 { 1 } else { 0 };

                // The length of unconsumed data needs to be updated into the reversed
                // completion buffer.
                self.buffers[self.index].1 = diff_size;
            }

            // Unconsumed data exists and is copied to the free buffer.
            #[allow(mutable_transmutes)]
            if len < size {
                unsafe { 
                    std::mem::transmute::<&[u8], &mut [u8]>(&self.buffers[self.index].0[..diff_size]) 
                }.copy_from_slice(&buffer[len..size]);
            }

            &buffer[..len]
        }
    }

    /// tcp socket process thread.
    ///
    /// This function is used to handle all connections coming from the tcp
    /// listener, and handle the receiving, sending and forwarding of messages.
    pub struct Server;

    impl ServerExt for Server {
        async fn start<T>(
            ServerStartOptions {
                bind,
                external,
                service,
                router,
                statistics,
            }: ServerStartOptions<T>,
        ) -> Result<(), anyhow::Error>
        where
            T: Clone + Observer + 'static,
        {
            let listener = TcpListener::bind(bind).await?;
            let local_addr = listener.local_addr()?;

            tokio::spawn(async move {
                // Accept all connections on the current listener, but exit the entire
                // process when an error occurs.
                while let Ok((socket, address)) = listener.accept().await {
                    let router = router.clone();
                    let reporter = statistics.get_reporter(Transport::TCP);
                    let mut receiver = router.get_receiver(address);
                    let mut operationer = service.get_operationer(address, external);

                    log::info!("tcp socket accept: addr={:?}, interface={:?}", address, local_addr,);

                    // Disable the Nagle algorithm.
                    // because to maintain real-time, any received data should be processed
                    // as soon as possible.
                    if let Err(e) = socket.set_nodelay(true) {
                        log::error!("tcp socket set nodelay failed!: addr={}, err={}", address, e);
                    }

                    let session_addr = SessionAddr {
                        interface: external,
                        address,
                    };

                    let (mut reader, writer) = socket.into_split();
                    let writer = Arc::new(Mutex::new(writer));

                    // Use a separate task to handle messages forwarded to this socket.
                    let writer_ = writer.clone();
                    let reporter_ = reporter.clone();
                    tokio::spawn(async move {
                        while let Some((bytes, method, _)) = receiver.recv().await {
                            let mut writer = writer_.lock().await;
                            if writer.write_all(bytes.as_slice()).await.is_err() {
                                break;
                            } else {
                                reporter_.send(
                                    &session_addr,
                                    &[Stats::SendBytes(bytes.len() as u32), Stats::SendPkts(1)],
                                );
                            }

                            // The channel data needs to be aligned in multiples of 4 in
                            // tcp. If the channel data is forwarded to tcp, the alignment
                            // bit needs to be filled, because if the channel data comes
                            // from udp, it is not guaranteed to be aligned and needs to be
                            // checked.
                            if method == ResponseMethod::ChannelData {
                                let pad = bytes.len() % 4;
                                if pad > 0 && writer.write_all(&ZERO_BYTES[..(4 - pad)]).await.is_err() {
                                    break;
                                }
                            }
                        }
                    });

                    let sessions = service.get_sessions();
                    tokio::spawn(async move {
                        let mut buffer = DoubleBufferQueue::default();

                        'a: while let Ok(size) = reader.read(&mut buffer).await {
                            // When the received message is 0, it means that the socket
                            // has been closed.
                            if size == 0 {
                                break;
                            } else {
                                reporter.send(&session_addr, &[Stats::ReceivedBytes(size as u32)]);
                                buffer.advance(size);
                            }

                            // The minimum length of a stun message will not be less
                            // than 4.
                            if buffer.len() < 4 {
                                continue;
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
                                    Ok(s) => {
                                        // Limit the maximum length of messages to 2048, this is to prevent buffer
                                        // overflow attacks.
                                        if s > 2048 {
                                            break 'a;
                                        }

                                        if s > buffer.len() {
                                            break;
                                        }

                                        reporter.send(&session_addr, &[Stats::ReceivedPkts(1)]);

                                        s
                                    }
                                };

                                let chunk = buffer.split(size);
                                if let Ok(ret) = operationer.route(chunk, address).await {
                                    if let Some(res) = ret {
                                        if let Some(ref inerface) = res.endpoint {
                                            router.send(
                                                inerface,
                                                res.method,
                                                res.relay.as_ref().unwrap_or(&address),
                                                res.bytes,
                                            );
                                        } else {
                                            if writer.lock().await.write_all(res.bytes).await.is_err() {
                                                break 'a;
                                            }

                                            reporter.send(
                                                &session_addr,
                                                &[Stats::SendBytes(res.bytes.len() as u32), Stats::SendPkts(1)],
                                            );

                                            if let ResponseMethod::Stun(method) = res.method {
                                                if method.is_error() {
                                                    reporter.send(&session_addr, &[Stats::ErrorPkts(1)]);
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    break 'a;
                                }
                            }
                        }

                        // When the tcp connection is closed, the procedure to close the session is
                        // process directly once, avoiding the connection being disconnected
                        // directly without going through the closing
                        // process.
                        sessions.refresh(&session_addr, 0);

                        router.remove(&address);

                        log::info!("tcp socket disconnect: addr={:?}, interface={:?}", address, local_addr);
                    });
                }

                log::error!("tcp server close: interface={:?}", local_addr);
            });

            Ok(())
        }
    }
}

/// start turn server.
///
/// create a specified number of threads,
/// each thread processes udp data separately.
pub async fn start<T>(config: &Config, statistics: &Statistics, service: &Service<T>) -> anyhow::Result<()>
where
    T: Clone + Observer + 'static,
{
    use crate::config::Transport;

    let router = Router::default();
    for Interface {
        transport,
        external,
        bind,
    } in config.turn.interfaces.iter().cloned()
    {
        let options = ServerStartOptions {
            statistics: statistics.clone(),
            service: service.clone(),
            router: router.clone(),
            external,
            bind,
        };

        match transport {
            Transport::UDP => udp::Server::start(options).await?,
            Transport::TCP => tcp::Server::start(options).await?,
        };

        log::info!(
            "turn server listening: bind={}, external={}, transport={:?}",
            bind,
            external,
            transport,
        );
    }

    Ok(())
}
