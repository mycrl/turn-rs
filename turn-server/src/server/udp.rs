use tokio::net::UdpSocket;
use turn_rs::Processor;
use std::{
    io::ErrorKind::*,
    sync::Arc,
};

use crate::monitor::{
    MonitorSender,
    Payload,
};

/// udp socket process thread.
///
/// read the data packet from the UDP socket and hand
/// it to the proto for processing, and send the processed
/// data packet to the specified address.
pub async fn processer(
    mut processor: Processor,
    sender: MonitorSender,
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
