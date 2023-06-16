use std::io::ErrorKind::ConnectionReset;
use faster_stun::Decoder;
use turn_rs::Processor;
use std::sync::Arc;
use tokio::io::{
    AsyncReadExt,
    AsyncWriteExt,
};

use bytes::BytesMut;
use tokio::net::{
    UdpSocket,
    TcpListener,
};

use crate::server::router::Router;
use crate::monitor::{
    MonitorSender,
    Payload,
};

pub async fn tcp_processor<T>(
    handle: T,
    sender: Arc<MonitorSender>,
    router: Arc<Router>,
    listen: TcpListener,
) where
    T: Fn() -> Processor,
{
    let local_addr = listen
        .local_addr()
        .expect("get tcp listener local addr failed!");

    while let Ok((mut socket, addr)) = listen.accept().await {
        let router = router.clone();
        let sender = sender.clone();
        let mut processor = handle();

        log::info!(
            "tcp socket accept: addr={:?}, interface={:?}",
            addr,
            local_addr,
        );

        tokio::spawn(async move {
            router.register(addr, addr).await;

            let (mut reader, mut writer) = socket.split();
            let mut receiver = router.get_receiver(addr).await;
            let mut buf = BytesMut::with_capacity(4096);

            loop {
                tokio::select! {
                    Ok(size) = reader.read_buf(&mut buf) => {
                        if size > 0 {
                            sender.send(Payload::Receive);
                            log::trace!(
                                "tcp socket receive: size={}, addr={:?}, interface={:?}",
                                size,
                                addr,
                                local_addr,
                            );

                            if let Ok(size) = Decoder::peek_size(&buf) {
                                let chunk = buf.split_to(size as usize);
                                if let Ok(Some((data, target))) = processor.process(&chunk, addr).await {
                                    if target.as_ref() != &addr && router.find(target.as_ref()).await {
                                        router.send(target.as_ref(), data).await;
                                    } else {
                                        if writer.write_all(&data).await.is_err() {
                                            break;
                                        }
                                    }

                                    sender.send(Payload::Send);
                                } else {
                                    sender.send(Payload::Failed);
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    Some((bytes, addr)) = receiver.recv() => {
                        if writer.write_all(&bytes).await.is_err() {
                            break;
                        }

                        log::trace!(
                            "tcp socket relay: size={}, addr={:?}",
                            bytes.len(),
                            addr,
                        );
                    }
                }
            }

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
    mut processor: Processor,
    sender: Arc<MonitorSender>,
    router: Arc<Router>,
    socket: Arc<UdpSocket>,
) -> anyhow::Result<()> {
    let local_addr = socket
        .local_addr()
        .expect("get udp socket local addr failed!");
    // let mut receiver = router.get_receiver(local_addr).await;
    let mut buf = vec![0u8; 4096];

    loop {
        tokio::select! {
            ret = socket.recv_from(&mut buf) => {
                let (size, addr) = match ret {
                    Ok(s) => s,
                    Err(e) => {
                        if e.kind() != ConnectionReset {
                            return Err(e.into());
                        } else {
                            continue;
                        }
                    },
                };

                // if !router.find(&addr).await {
                //     router.register(local_addr, addr).await;
                // }

                sender.send(Payload::Receive);
                log::trace!(
                    "udp socket receive: size={}, addr={:?}, interface={:?}",
                    size,
                    addr,
                    local_addr
                );

                // The stun message requires at least 4 bytes. (currently the smallest
                // stun message is channel data, excluding content)
                if size >= 4 {
                    if let Ok(Some((bytes, target))) =
                        processor.process(&buf[..size], addr).await
                    {
                        if &addr != target.as_ref() && router.find(target.as_ref()).await {
                            router.send(target.as_ref(), bytes).await;
                        } else {
                            if let Err(e) = socket.send_to(bytes, target.as_ref()).await {
                                if e.kind() != ConnectionReset {
                                    return Err(e.into());
                                }
                            } else {
                                sender.send(Payload::Send);
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

                sender.send(Payload::Failed);
                log::trace!(
                    "udp socket process failed: size={}, addr={:?}",
                    size,
                    addr
                );
            }
            // Some((bytes, addr)) = receiver.recv() => {
            //     if let Err(e) = socket.send_to(&bytes, addr).await {
            //         if e.kind() != ConnectionReset {
            //             return Err(e.into());
            //         }
            //     } else {
            //         sender.send(Payload::Send);
            //         log::trace!(
            //             "udp socket relay: size={}, addr={:?}",
            //             bytes.len(),
            //             addr,
            //         );
            //     }
            // }
        }
    }
}
