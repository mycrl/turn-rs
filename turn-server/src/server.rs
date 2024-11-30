use crate::{
    config::{Config, Interface, Transport},
    router::Router,
    statistics::{Statistics, Stats},
};
use std::{
    fs::File,
    io::{BufReader, ErrorKind::ConnectionReset},
    net::SocketAddr,
    sync::Arc,
};

use bytes::BytesMut;
use rustls::{pki_types, ServerConfig};
use stun::Decoder;
use tokio::{
    io::AsyncReadExt,
    io::AsyncWriteExt,
    net::{TcpListener, UdpSocket},
    sync::Mutex,
};
use tokio_rustls::TlsAcceptor;
use turn::{Service, StunClass};

/// start turn server.
///
/// create a specified number of threads,
/// each thread processes udp data separately.
pub async fn run(
    config: Arc<Config>,
    statistics: Statistics,
    service: &Service,
) -> anyhow::Result<()> {
    let router = Arc::new(Router::default());
    for Interface {
        transport,
        external,
        bind,
        cert_file,
        key_file,
    } in config.turn.interfaces.clone()
    {
        match transport {
            Transport::UDP => {
                tokio::spawn(udp_server(
                    UdpSocket::bind(bind).await?,
                    external,
                    service.clone(),
                    router.clone(),
                    statistics.clone(),
                ));
            }
            Transport::TCP => {
                tokio::spawn(tcp_server(
                    TcpListener::bind(bind).await?,
                    external,
                    service.clone(),
                    router.clone(),
                    statistics.clone(),
                ));
            }
            Transport::TLS => {
                let cert_file =
                    cert_file.ok_or_else(|| anyhow::anyhow!("TLS cert_file not specified"))?;
                let key_file =
                    key_file.ok_or_else(|| anyhow::anyhow!("TLS key_file not specified"))?;

                let certs = load_certs(&cert_file)?;
                let key = load_private_key(&key_file)?;
                let config = ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(certs, key)?;

                tokio::spawn(tls_server(
                    TcpListener::bind(bind).await?,
                    config,
                    external,
                    service.clone(),
                    router.clone(),
                    statistics.clone(),
                ));
            }
            Transport::DTLS => {
                let cert_file =
                    cert_file.ok_or_else(|| anyhow::anyhow!("DTLS cert_file not specified"))?;
                let key_file =
                    key_file.ok_or_else(|| anyhow::anyhow!("DTLS key_file not specified"))?;

                let certs = load_certs(&cert_file)?;
                let key = load_private_key(&key_file)?;
                let config = ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(certs, key)?;

                tokio::spawn(dtls_server(
                    UdpSocket::bind(bind).await?,
                    config,
                    external,
                    service.clone(),
                    router.clone(),
                    statistics.clone(),
                ));
            }
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
    service: Service,
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
                    if let Ok(Some(res)) = operationer.process(&chunk, addr).await {
                        let target = res.relay.unwrap_or(addr);
                        if let Some(to) = res.interface {
                            router.send(&to, res.kind, &target, res.data);
                        } else {
                            if writer.lock().await.write_all(res.data).await.is_err() {
                                break 'a;
                            }

                            reporter.send(
                                Transport::TCP,
                                &addr,
                                &[Stats::SendBytes(res.data.len() as u64), Stats::SendPkts(1)],
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
    service: Service,
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
                    if let Ok(Some(res)) = operationer.process(&buf[..size], addr).await {
                        let target = res.relay.unwrap_or(addr);
                        if let Some(to) = res.interface {
                            router.send(&to, res.kind, &target, res.data);
                        } else {
                            if let Err(e) = socket.send_to(res.data, &target).await {
                                if e.kind() != ConnectionReset {
                                    break;
                                }
                            }

                            reporter.send(
                                Transport::UDP,
                                &addr,
                                &[Stats::SendBytes(res.data.len() as u64), Stats::SendPkts(1)],
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

fn load_certs(path: &str) -> anyhow::Result<Vec<pki_types::CertificateDer<'static>>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    Ok(certs)
}

fn load_private_key(path: &str) -> anyhow::Result<pki_types::PrivateKeyDer<'static>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    match rustls_pemfile::private_key(&mut reader)? {
        Some(key) => Ok(key),
        None => Err(anyhow::anyhow!("no private key found")),
    }
}

async fn tls_server(
    listen: TcpListener,
    config: ServerConfig,
    external: SocketAddr,
    service: Service,
    router: Arc<Router>,
    statistics: Statistics,
) {
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let local_addr = listen
        .local_addr()
        .expect("get tls listener local addr failed!");

    while let Ok((socket, addr)) = listen.accept().await {
        let acceptor = acceptor.clone();
        let router = router.clone();
        let reporter = statistics.get_reporter();
        let mut receiver = router.get_receiver(addr);
        let mut operationer = service.get_operationer(addr, external);

        log::info!(
            "tls socket accept: addr={:?}, interface={:?}",
            addr,
            local_addr,
        );

        match acceptor.accept(socket).await {
            Ok(tls_stream) => {
                let (mut reader, writer) = tokio::io::split(tls_stream);
                let writer = Arc::new(Mutex::new(writer));
                let writer_ = writer.clone();
                let reporter_ = reporter.clone();

                tokio::spawn(async move {
                    while let Some((bytes, kind, _)) = receiver.recv().await {
                        let mut writer = writer_.lock().await;
                        if writer.write_all(bytes.as_slice()).await.is_err() {
                            break;
                        } else {
                            reporter_.send(
                                Transport::TLS,
                                &addr,
                                &[Stats::SendBytes(bytes.len() as u64), Stats::SendPkts(1)],
                            );
                        }

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
                        if size == 0 {
                            break;
                        } else {
                            reporter.send(
                                Transport::TLS,
                                &addr,
                                &[Stats::ReceivedBytes(size as u64)],
                            );
                        }

                        if buf.len() < 4 {
                            continue;
                        }

                        loop {
                            if buf.len() <= 4 {
                                break;
                            }

                            let size = match Decoder::message_size(&buf, true) {
                                Ok(s) if s > buf.len() => break,
                                Err(_) => break,
                                Ok(s) => {
                                    reporter.send(Transport::TLS, &addr, &[Stats::ReceivedPkts(1)]);
                                    s
                                }
                            };

                            let chunk = buf.split_to(size);
                            if let Ok(Some(res)) = operationer.process(&chunk, addr).await {
                                let target = res.relay.unwrap_or(addr);
                                if let Some(to) = res.interface {
                                    router.send(&to, res.kind, &target, res.data);
                                } else {
                                    if writer.lock().await.write_all(res.data).await.is_err() {
                                        break 'a;
                                    }

                                    reporter.send(
                                        Transport::TLS,
                                        &addr,
                                        &[
                                            Stats::SendBytes(res.data.len() as u64),
                                            Stats::SendPkts(1),
                                        ],
                                    );
                                }
                            } else {
                                reporter.send(Transport::TLS, &addr, &[Stats::ErrorPkts(1)]);
                            }
                        }
                    }

                    router.remove(&addr);
                    log::info!(
                        "tls socket disconnect: addr={:?}, interface={:?}",
                        addr,
                        local_addr,
                    );
                });
            }
            Err(e) => {
                log::error!("tls handshake failed: addr={}, err={}", addr, e);
            }
        }
    }

    log::error!("tls server close: interface={:?}", local_addr);
}

async fn dtls_server(
    socket: UdpSocket,
    _config: ServerConfig,
    _external: SocketAddr,
    _service: Service,
    _router: Arc<Router>,
    _statistics: Statistics,
) {
    let socket = Arc::new(socket);
    let _local_addr = socket
        .local_addr()
        .expect("get dtls socket local addr failed!");

    // TODO:DTLS implementation goes here
    // Note: DTLS implementation requires additional work with tokio-dtls
    log::warn!("DTLS support is not fully implemented yet");
}
