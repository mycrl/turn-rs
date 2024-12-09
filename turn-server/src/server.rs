use crate::{
    config::{Config, Interface, Transport},
    observer::Observer,
    router::Router,
    statistics::{Statistics, Stats},
};

use std::{io::ErrorKind::ConnectionReset, net::SocketAddr, sync::Arc};

use bytes::BytesMut;
use stun::Decoder;
use tokio::{
    io::AsyncReadExt,
    io::AsyncWriteExt,
    net::{TcpListener, UdpSocket},
    sync::Mutex,
};

use turn::{Service, StunClass};

/// start turn server.
///
/// create a specified number of threads,
/// each thread processes udp data separately.
pub async fn run(
    config: Arc<Config>,
    statistics: Statistics,
    service: &Service<Observer>,
) -> anyhow::Result<()> {
    let router = Arc::new(Router::default());
    for Interface {
        transport,
        external,
        bind,
    } in config.turn.interfaces.clone()
    {
        if transport == Transport::UDP {
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

static ZERO_BUF: [u8; 4] = [0u8; 4];

/// tcp socket process thread.
///
/// This function is used to handle all connections coming from the tcp
/// listener, and handle the receiving, sending and forwarding of messages.
async fn tcp_server(
    listen: TcpListener,
    external: SocketAddr,
    service: Service<Observer>,
    router: Arc<Router>,
    statistics: Statistics,
) {
    let local_addr = listen
        .local_addr()
        .expect("get tcp listener local addr failed!");

    // Accept all connections on the current listener, but exit the entire
    // process when an error occurs.
    while let Ok((socket, addr)) = listen.accept().await {
        let router = router.clone();
        let reporter = statistics.get_reporter();
        let mut receiver = router.get_receiver(addr);
        let mut operationer = service.get_operationer(addr, external);

        log::info!(
            "tcp socket accept: addr={:?}, interface={:?}",
            addr,
            local_addr,
        );

        // Disable the Nagle algorithm.
        // because to maintain real-time, any received data should be processed
        // as soon as possible.
        if let Err(e) = socket.set_nodelay(true) {
            log::error!("tcp socket set nodelay failed!: addr={}, err={}", addr, e);
        }

        let (mut reader, writer) = socket.into_split();
        let writer = Arc::new(Mutex::new(writer));
        let writer_ = writer.clone();
        let reporter_ = reporter.clone();

        // Use a separate task to handle messages forwarded to this socket.
        tokio::spawn(async move {
            while let Some((bytes, kind, _)) = receiver.recv().await {
                let mut writer = writer_.lock().await;
                if writer.write_all(bytes.as_slice()).await.is_err() {
                    break;
                } else {
                    reporter_.send(
                        Transport::TCP,
                        &addr,
                        &[Stats::SendBytes(bytes.len() as u64), Stats::SendPkts(1)],
                    );
                }

                // The channel data needs to be aligned in multiples of 4 in
                // tcp. If the channel data is forwarded to tcp, the alignment
                // bit needs to be filled, because if the channel data comes
                // from udp, it is not guaranteed to be aligned and needs to be
                // checked.
                if kind == StunClass::Channel {
                    let pad = bytes.len() % 4;
                    if pad > 0 && writer.write_all(&ZERO_BUF[..(4 - pad)]).await.is_err() {
                        break;
                    }
                }
            }
        });

        tokio::spawn(async move {
            let mut buf = BytesMut::new();

            'a: while let Ok(size) = reader.read_buf(&mut buf).await {
                // When the received message is 0, it means that the socket
                // has been closed.
                if size == 0 {
                    break;
                } else {
                    reporter.send(Transport::TCP, &addr, &[Stats::ReceivedBytes(size as u64)]);
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
                            reporter.send(Transport::TCP, &addr, &[Stats::ReceivedPkts(1)]);

                            s
                        }
                    };

                    let chunk = buf.split_to(size);
                    if let Ok(Some(res)) = operationer.route(&chunk, addr).await {
                        let target = res.relay.map(|it| it.address).unwrap_or(addr);
                        if let Some(relay) = res.relay {
                            router.send(&relay.interface, res.kind, &target, res.bytes);
                        } else {
                            if writer.lock().await.write_all(res.bytes).await.is_err() {
                                break 'a;
                            }

                            reporter.send(
                                Transport::TCP,
                                &addr,
                                &[Stats::SendBytes(res.bytes.len() as u64), Stats::SendPkts(1)],
                            );
                        }
                    } else {
                        reporter.send(Transport::TCP, &addr, &[Stats::ErrorPkts(1)]);
                    }
                }
            }

            router.remove(&addr);
            log::info!(
                "tcp socket disconnect: addr={:?}, interface={:?}",
                addr,
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
async fn udp_server(
    socket: UdpSocket,
    external: SocketAddr,
    service: Service<Observer>,
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
        let reporter = statistics.get_reporter();
        let mut operationer = service.get_operationer(external, external);

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

                reporter.send(
                    Transport::UDP,
                    &addr,
                    &[Stats::ReceivedBytes(size as u64), Stats::ReceivedPkts(1)],
                );

                // The stun message requires at least 4 bytes. (currently the
                // smallest stun message is channel data,
                // excluding content)
                if size >= 4 {
                    if let Ok(Some(res)) = operationer.route(&buf[..size], addr).await {
                        let target = res.relay.map(|it| it.address).unwrap_or(addr);
                        if let Some(relay) = res.relay {
                            router.send(&relay.interface, res.kind, &target, res.bytes);
                        } else {
                            if let Err(e) = socket.send_to(res.bytes, &target).await {
                                if e.kind() != ConnectionReset {
                                    break;
                                }
                            }

                            reporter.send(
                                Transport::UDP,
                                &addr,
                                &[Stats::SendBytes(res.bytes.len() as u64), Stats::SendPkts(1)],
                            );
                        }
                    } else {
                        reporter.send(Transport::UDP, &addr, &[Stats::ErrorPkts(1)]);
                    }
                }
            }
        });
    }

    let reporter = statistics.get_reporter();
    let mut receiver = router.get_receiver(external);
    while let Some((bytes, _, addr)) = receiver.recv().await {
        if let Err(e) = socket.send_to(&bytes, addr).await {
            if e.kind() != ConnectionReset {
                break;
            }
        } else {
            reporter.send(
                Transport::UDP,
                &addr,
                &[Stats::SendBytes(bytes.len() as u64), Stats::SendPkts(1)],
            );
        }
    }

    router.remove(&external);
    log::error!("udp server close: interface={:?}", local_addr);
}
