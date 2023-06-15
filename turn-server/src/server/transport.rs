use std::io::ErrorKind::ConnectionReset;
use turn_rs::Processor;
use std::sync::Arc;
use tokio::io::{
    AsyncReadExt,
    AsyncWriteExt,
};

use tokio::net::{
    UdpSocket,
    TcpListener,
};

use crate::server::router::Router;
use crate::monitor::{
    MonitorSender,
    Payload,
};

/// udp socket process thread.
///
/// read the data packet from the UDP socket and hand
/// it to the proto for processing, and send the processed
/// data packet to the specified address.
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

    // udp socket process thread.
    //
    // read the data packet from the UDP socket and hand
    // it to the proto for processing, and send the processed
    // data packet to the specified address.
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
            let (mut reader, mut writer) = socket.split();
            let mut receiver = router.get_receiver(addr.clone()).await;
            let mut buf = [0u8; 4096];

            loop {
                tokio::select! {
                    Ok(size) = reader.read(&mut buf) => {
                        if size > 0 {
                            sender.send(Payload::Receive);
                            log::trace!(
                                "tcp socket receive: size={}, addr={:?}, interface={:?}",
                                size,
                                addr,
                                local_addr,
                            );

                            // udp socket process thread.
                            //
                            // read the data packet from the UDP socket and hand
                            // it to the proto for processing, and send the processed
                            // data packet to the specified address.
                            if let Ok(Some((data, addr))) = processor.process(&buf[..size], addr).await {
                                router.send(addr.as_ref(), data, true).await;
                                sender.send(Payload::Send);
                            } else {
                                sender.send(Payload::Failed);
                            }
                        } else {
                            break;
                        }
                    }
                    Some(bytes) = receiver.recv() => {
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
    let mut buf = vec![0u8; 4096];

    loop {
        // TODO: An error will also be reported when the remote host is shut
        // down, which is not processed yet, but a warning will be
        // issued.
        let (size, addr) = match socket.recv_from(&mut buf).await {
            Ok(s) => s,
            Err(e) => {
                if e.kind() != ConnectionReset {
                    return Err(e.into());
                } else {
                    continue;
                }
            },
        };

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
            if let Ok(Some((res, addr))) =
                processor.process(&buf[..size], addr).await
            {
                if let Err(e) = socket.send_to(res, addr.as_ref()).await {
                    if e.kind() != ConnectionReset {
                        return Err(e.into());
                    }
                } else {
                    sender.send(Payload::Send);
                    log::trace!(
                        "udp socket relay: size={}, addr={:?}",
                        res.len(),
                        addr.as_ref()
                    );
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
}
