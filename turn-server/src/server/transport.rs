use crate::config::Interface;
use super::router::Router;
use faster_stun::Decoder;
use bytes::BytesMut;
use std::{
    io::ErrorKind::ConnectionReset,
    sync::Arc,
};

use tokio::{
    io::AsyncReadExt,
    io::AsyncWriteExt,
    sync::Mutex,
};

use turn_rs::{
    Processor,
    Service,
};

use tokio::net::{
    UdpSocket,
    TcpListener,
};

/// tcp socket process thread.
///
/// This function is used to handle all connections coming from the tcp
/// listener, and handle the receiving, sending and forwarding of messages.
pub async fn tcp_processor<T>(
    listen: TcpListener,
    handle: T,
    router: Arc<Router>,
) where
    T: Fn(u8) -> Processor,
{
    let local_addr = listen
        .local_addr()
        .expect("get tcp listener local addr failed!");

    // Accept all connections on the current listener, but exit the entire
    // process when an error occurs.
    while let Ok((socket, addr)) = listen.accept().await {
        let router = router.clone();
        let (index, mut receiver) = router.get_receiver().await;
        let mut processor = handle(index);

        log::info!(
            "tcp socket accept: addr={:?}, interface={:?}",
            addr,
            local_addr,
        );

        // Disable the Nagle algorithm.
        // because to maintain real-time, any received data should be processed
        // as soon as possible.
        if let Err(e) = socket.set_nodelay(true) {
            log::error!(
                "tcp socket set nodelay failed!: addr={}, err={}",
                addr,
                e
            );
        }

        let (mut reader, writer) = socket.into_split();
        let writer = Arc::new(Mutex::new(writer));
        let writer_ = writer.clone();

        // Use a separate task to handle messages forwarded to this socket.
        tokio::spawn(async move {
            while let Some((bytes, target)) = receiver.recv().await {
                if writer_
                    .lock()
                    .await
                    .write_all(bytes.as_slice())
                    .await
                    .is_err()
                {
                    break;
                } else {
                    log::trace!(
                        "tcp socket relay: size={}, addr={:?}",
                        bytes.len(),
                        target,
                    );
                }
            }
        });

        tokio::spawn(async move {
            let mut buf = BytesMut::new();
            'a: loop {
                if let Ok(size) = reader.read_buf(&mut buf).await {
                    // When the received message is 0, it means that the socket
                    // has been closed.
                    if size == 0 {
                        break;
                    }

                    // The minimum length of a stun message will not be less
                    // than 4.
                    if buf.len() < 4 {
                        continue;
                    }

                    log::trace!(
                        "tcp socket receive: size={}, addr={:?}, \
                         interface={:?}",
                        size,
                        addr,
                        local_addr,
                    );

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
                            Ok(s) => s,
                        };

                        let chunk = buf.split_to(size);
                        if let Ok(Some((data, target))) =
                            processor.process(&chunk, addr).await
                        {
                            if let Some((target, index)) = target {
                                router.send(index, &target, data).await;
                            } else {
                                if writer
                                    .lock()
                                    .await
                                    .write_all(&data)
                                    .await
                                    .is_err()
                                {
                                    break 'a;
                                }
                            }
                        }
                    }
                } else {
                    break;
                }
            }

            router.remove(index).await;
            log::info!(
                "tcp socket disconnect: addr={:?}, interface={:?}",
                addr,
                local_addr,
            );
        });
    }
}

/// udp socket process thread.
///
/// read the data packet from the UDP socket and hand
/// it to the proto for processing, and send the processed
/// data packet to the specified address.
pub async fn udp_processor(
    socket: UdpSocket,
    interface: Interface,
    service: Service,
    router: Arc<Router>,
) {
    let socket = Arc::new(socket);
    let local_addr = socket
        .local_addr()
        .expect("get udp socket local addr failed!");
    let (index, mut receiver) = router.get_receiver().await;

    for _ in 0..10 {
        let socket = socket.clone();
        let router = router.clone();
        let mut processor = service.get_processor(index, interface.external);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];

            loop {
                // TODO: An error will also be reported when the remote host is
                // shut down, which is not processed yet, but a
                // warning will be issued.
                let (size, addr) = match socket.recv_from(&mut buf).await {
                    Err(e) if e.kind() != ConnectionReset => break,
                    Ok(s) => s,
                    _ => continue,
                };

                log::trace!(
                    "udp socket receive: size={}, addr={:?}, interface={:?}",
                    size,
                    addr,
                    local_addr
                );

                // The stun message requires at least 4 bytes. (currently the
                // smallest stun message is channel data,
                // excluding content)
                if size >= 4 {
                    if let Ok(Some((bytes, target))) =
                        processor.process(&buf[..size], addr).await
                    {
                        if let Some((target, index)) = target {
                            router.send(index, &target, bytes).await;
                        } else {
                            if let Err(e) = socket.send_to(bytes, &addr).await {
                                if e.kind() != ConnectionReset {
                                    break;
                                }
                            } else {
                                log::trace!(
                                    "udp socket relay: size={}, addr={:?}",
                                    bytes.len(),
                                    target.as_ref()
                                );
                            }
                        }

                        continue;
                    }
                }

                log::trace!(
                    "udp socket process failed: size={}, addr={:?}",
                    size,
                    addr
                );
            }

            router.remove(processor.index).await;
            log::info!("udp server close: interface={:?}", local_addr,);
        });
    }

    while let Some((bytes, addr)) = receiver.recv().await {
        if let Err(e) = socket.send_to(&bytes, addr).await {
            if e.kind() != ConnectionReset {
                break;
            }
        }
    }
}
