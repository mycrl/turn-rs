use crate::{
    config::Transport,
    server::{OutboundType, TransportOptions},
    statistics::Stats,
};

use std::{io::ErrorKind::ConnectionReset, net::UdpSocket, sync::Arc, thread};

use bytes::{Bytes, BytesMut};
use service::{
    ServiceHandler,
    forwarding::{ForwardResult, Outbound},
    session::Identifier,
};

/// udp server
///
/// read the data packet from the UDP socket and hand
/// it to the proto for processing, and send the processed
/// data packet to the specified address.
pub async fn listener<T>(
    TransportOptions {
        listen,
        external,
        service,
        exchanger,
        statistics,
        config,
    }: TransportOptions<'_, T>,
) -> Result<(), anyhow::Error>
where
    T: Clone + ServiceHandler + 'static,
{
    let runtime = config.runtime.clone();

    let socket = Arc::new(UdpSocket::bind(listen)?);
    let local_addr = socket.local_addr()?;

    // Try to bind to a core; if binding fails, fall back to the normal thread group
    for core_id in core_affinity::get_core_ids()
        .map(|items| {
            items
                .into_iter()
                .take(runtime.max_threads)
                .map(Some)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| (0..runtime.max_threads).map(|_| None).collect::<Vec<_>>())
    {
        let socket = socket.clone();
        let exchanger = exchanger.clone();
        let reporter = statistics.get_reporter(Transport::Udp);
        let mut forwarder = service.get_forwarder(external, external);

        thread::spawn(move || {
            if let Some(core_id) = core_id {
                let _ = core_affinity::set_for_current(core_id);
            }

            let mut id = Identifier {
                // Placeholder, will be changed to the real address later
                source: external,
                interface: external,
            };

            let mut buffer = BytesMut::zeroed(runtime.mtu * 2);

            loop {
                // Note: An error will also be reported when the remote host is
                // shut down, which is not processed yet, but a
                // warning will be issued.
                let (size, addr) = match socket.recv_from(&mut buffer) {
                    Err(e) if e.kind() != ConnectionReset => break,
                    Ok((size, addr)) => {
                        id.source = addr;

                        (size, addr)
                    }
                    _ => continue,
                };

                reporter.send(&id, &[Stats::ReceivedBytes(size), Stats::ReceivedPkts(1)]);

                // The stun message requires at least 4 bytes. (currently the
                // smallest stun message is channel data,
                // excluding content)
                if size < 4 {
                    continue;
                }

                if let ForwardResult::Outbound(outbound) = forwarder.forward(&buffer[..size], addr)
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

                    let to = target.relay.as_ref().unwrap_or(&addr);
                    if let Some(ref endpoint) = target.endpoint {
                        exchanger.send(endpoint, ty, to, Bytes::copy_from_slice(bytes));
                    } else {
                        if let Err(e) = socket.send_to(bytes, to) {
                            if e.kind() != ConnectionReset {
                                break;
                            }
                        }

                        reporter.send(&id, &[Stats::SendBytes(bytes.len()), Stats::SendPkts(1)]);

                        if let OutboundType::Message(method) = ty {
                            if method.is_error() {
                                reporter.send(&id, &[Stats::ErrorPkts(1)]);
                            }
                        }
                    }
                }
            }
        });
    }

    tokio::spawn(async move {
        let mut id = Identifier {
            source: external,
            interface: external,
        };

        let reporter = statistics.get_reporter(Transport::Udp);
        let mut receiver = exchanger.get_receiver(external);
        while let Some((bytes, _, addr)) = receiver.recv().await {
            id.source = addr;

            if let Err(e) = socket.send_to(&bytes, addr) {
                if e.kind() != ConnectionReset {
                    break;
                }
            } else {
                reporter.send(&id, &[Stats::SendBytes(bytes.len()), Stats::SendPkts(1)]);
            }
        }

        exchanger.remove(&external);

        log::error!("udp server close: interface={local_addr:?}");
    });

    log::info!("turn server listening: listen={listen}, external={external}, transport=UDP");

    Ok(())
}
