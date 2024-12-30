use crate::{
    config::{Config, Interface},
    router::Router,
    statistics::{Statistics, Stats},
};

use std::{io::ErrorKind::ConnectionReset, net::SocketAddr, sync::Arc};

use bytes::BytesMut;
use stun::{Decoder, Transport};
use tokio::{
    io::AsyncReadExt,
    io::AsyncWriteExt,
    net::{TcpListener, UdpSocket},
    sync::Mutex,
};

use turn::{Observer, ResponseMethod, Service, Socket};

static ZERO_BUF: [u8; 4] = [0u8; 4];

/// tcp socket process thread.
///
/// This function is used to handle all connections coming from the tcp
/// listener, and handle the receiving, sending and forwarding of messages.
async fn tcp_server<T: Clone + Observer + 'static>(
    listen: TcpListener,
    external: SocketAddr,
    service: Service<T>,
    router: Arc<Router>,
    statistics: Statistics,
) {
    let local_addr = listen
        .local_addr()
        .expect("get tcp listener local addr failed!");

    // Accept all connections on the current listener, but exit the entire
    // process when an error occurs.
    while let Ok((socket, address)) = listen.accept().await {
        let router = router.clone();
        let reporter = statistics.get_reporter(Transport::TCP);
        let mut receiver = router.get_receiver(address);
        let mut operationer = service.get_operationer(address, external);

        log::info!(
            "tcp socket accept: addr={:?}, interface={:?}",
            address,
            local_addr,
        );

        // Disable the Nagle algorithm.
        // because to maintain real-time, any received data should be processed
        // as soon as possible.
        if let Err(e) = socket.set_nodelay(true) {
            log::error!(
                "tcp socket set nodelay failed!: addr={}, err={}",
                address,
                e
            );
        }

        let sock_addr = Socket {
            interface: external,
            address,
        };

        let (mut reader, writer) = socket.into_split();
        let writer = Arc::new(Mutex::new(writer));
        let writer_ = writer.clone();
        let reporter_ = reporter.clone();

        // Use a separate task to handle messages forwarded to this socket.
        tokio::spawn(async move {
            while let Some((bytes, method, _)) = receiver.recv().await {
                let mut writer = writer_.lock().await;
                if writer.write_all(bytes.as_slice()).await.is_err() {
                    break;
                } else {
                    reporter_.send(
                        &sock_addr,
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
                    if pad > 0 && writer.write_all(&ZERO_BUF[..(4 - pad)]).await.is_err() {
                        break;
                    }
                }
            }
        });

        let sessions = service.get_sessions();
        tokio::spawn(async move {
            let mut buf = BytesMut::new();

            'a: while let Ok(size) = reader.read_buf(&mut buf).await {
                // When the received message is 0, it means that the socket
                // has been closed.
                if size == 0 {
                    break;
                } else {
                    reporter.send(&sock_addr, &[Stats::ReceivedBytes(size as u32)]);
                }

                // The minimum length of a stun message will not be less
                // than 4.
                if buf.len() < 4 {
                    continue;
                }

                loop {
                    if buf.len() <= 4 {
                        break;
                    }

                    // Try to get the message length, if the currently
                    // received data is less than the message length, jump
                    // out of the current loop and continue to receive more
                    // data.
                    let size = match Decoder::message_size(&buf, true) {
                        Ok(s) if s > buf.len() => break,
                        Err(_) => break,
                        Ok(s) => {
                            reporter.send(&sock_addr, &[Stats::ReceivedPkts(1)]);

                            s
                        }
                    };

                    let chunk = buf.split_to(size);
                    if let Ok(ret) = operationer.route(&chunk, address).await {
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
                                    &sock_addr,
                                    &[Stats::SendBytes(res.bytes.len() as u32), Stats::SendPkts(1)],
                                );

                                if let ResponseMethod::Stun(method) = res.method {
                                    if method.is_error() {
                                        reporter.send(&sock_addr, &[Stats::ErrorPkts(1)]);
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
            // process directly once, avoiding the connection being disconnected directly
            // without going through the closing process.
            sessions.refresh(&sock_addr, 0);

            router.remove(&address);

            log::info!(
                "tcp socket disconnect: addr={:?}, interface={:?}",
                address,
                local_addr,
            );
        });
    }

    log::error!("tcp server close: interface={:?}", local_addr);
}

/// udp socket process thread.
///
/// read the data packet from the UDP socket and hand
/// it to the proto for processing, and send the processed
/// data packet to the specified address.
async fn udp_server<T: Clone + Observer + 'static>(
    socket: UdpSocket,
    external: SocketAddr,
    service: Service<T>,
    router: Arc<Router>,
    statistics: Statistics,
) {
    let socket = Arc::new(socket);
    let local_addr = socket
        .local_addr()
        .expect("get udp socket local addr failed!");

    for _ in 0..num_cpus::get() {
        let socket = socket.clone();
        let router = router.clone();
        let reporter = statistics.get_reporter(Transport::UDP);
        let mut operationer = service.get_operationer(external, external);

        let mut sock_addr = Socket {
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

                sock_addr.address = addr;

                reporter.send(
                    &sock_addr,
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
                                &sock_addr,
                                &[Stats::SendBytes(res.bytes.len() as u32), Stats::SendPkts(1)],
                            );

                            if let ResponseMethod::Stun(method) = res.method {
                                if method.is_error() {
                                    reporter.send(&sock_addr, &[Stats::ErrorPkts(1)]);
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    {
        let mut sock_addr = Socket {
            address: external,
            interface: external,
        };

        let reporter = statistics.get_reporter(Transport::UDP);
        let mut receiver = router.get_receiver(external);
        while let Some((bytes, _, addr)) = receiver.recv().await {
            sock_addr.address = addr;

            if let Err(e) = socket.send_to(&bytes, addr).await {
                if e.kind() != ConnectionReset {
                    break;
                }
            } else {
                reporter.send(
                    &sock_addr,
                    &[Stats::SendBytes(bytes.len() as u32), Stats::SendPkts(1)],
                );
            }
        }

        router.remove(&external);
    }

    log::error!("udp server close: interface={:?}", local_addr);
}

/// start turn server.
///
/// create a specified number of threads,
/// each thread processes udp data separately.
pub async fn run<T: Clone + Observer + 'static>(
    config: Arc<Config>,
    statistics: Statistics,
    service: &Service<T>,
) -> anyhow::Result<()> {
    let router = Arc::new(Router::default());
    for Interface {
        transport,
        external,
        bind,
    } in config.turn.interfaces.clone()
    {
        if transport == crate::config::Transport::UDP {
            tokio::spawn(udp_server(
                UdpSocket::bind(bind).await?,
                external,
                service.clone(),
                router.clone(),
                statistics.clone(),
            ));
        } else {
            tokio::spawn(tcp_server(
                TcpListener::bind(bind).await?,
                external,
                service.clone(),
                router.clone(),
                statistics.clone(),
            ));
        }

        log::info!(
            "turn server listening: addr={}, external={}, transport={:?}",
            bind,
            external,
            transport,
        );
    }

    Ok(())
}
