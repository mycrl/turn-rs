use std::io::ErrorKind::ConnectionReset;
use std::sync::Arc;
use bytes::BytesMut;
use faster_stun::Decoder;
use tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
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

use super::router::Router;
use crate::config::Interface;

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

    while let Ok((socket, addr)) = listen.accept().await {
        let router = router.clone();
        let (index, mut receiver) = router.get_receiver().await;
        let mut processor = handle(index);

        log::info!(
            "tcp socket accept: addr={:?}, interface={:?}",
            addr,
            local_addr,
        );

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

        tokio::spawn(async move {
            while let Some((bytes, target)) = receiver.recv().await {
                if writer_.lock().await.write_all(&bytes).await.is_err() {
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
            'a: while let Ok(size) = reader.read_buf(&mut buf).await {
                if size == 0 {
                    break;
                }

                println!("== {}, {}", size, buf.len());
                if buf.len() < 4 {
                    continue;
                }

                // log::trace!(
                //     "tcp socket receive: size={}, addr={:?}, interface={:?}",
                //     size,
                //     addr,
                //     local_addr,
                // );

                loop {
                    if buf.len() <= 4 {
                        break;
                    }

                    let size = match Decoder::message_size(&buf) {
                        Ok(s) if s > buf.len() => break,
                        Err(_) => break,
                        Ok(s) => s,
                    };

                    println!("{}, {}", size, buf.len());
                    if size == 84 {
                        println!("{:?}", &buf[..]);
                    }

                    let chunk = buf.split_to(size);
                    let ret = processor.process(&chunk, addr).await;
                    if ret.is_err() {
                        std::process::exit(1);
                    }

                    if let Ok(Some((data, target))) =
                        ret
                    {
                        if let Some((target, index)) = target {
                            router.send(index, &target, data).await;
                        } else {
                            if writer.lock().await.write_all(&data).await.is_err() {
                                break 'a;
                            }
                        }
                    }
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
