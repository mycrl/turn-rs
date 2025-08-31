use crate::{
    server::{MAX_MESSAGE_SIZE, OutboundType, TransportOptions, buffer::ExchangeBuffer},
    statistics::Stats,
};

use codec::{Decoder, message::attributes::Transport};
use service::{
    ServiceHandler,
    forwarding::{ForwardResult, Outbound},
    session::Identifier,
};

use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpListener};

/// tcp server
///
/// This function is used to handle all connections coming from the tcp
/// listener, and handle the receiving, sending and forwarding of messages.
pub async fn listener<T>(
    TransportOptions {
        listen,
        external,
        service,
        exchanger,
        statistics,
        ..
    }: TransportOptions<'_, T>,
) -> Result<(), anyhow::Error>
where
    T: Clone + ServiceHandler + 'static,
{
    let listener = TcpListener::bind(listen).await?;
    let local_addr = listener.local_addr()?;

    tokio::spawn(async move {
        // Accept all connections on the current listener, but exit the entire
        // process when an error occurs.
        while let Ok((mut socket, address)) = listener.accept().await {
            let exchanger = exchanger.clone();
            let reporter = statistics.get_reporter(Transport::TCP);
            let mut receiver = exchanger.get_receiver(address);
            let mut forwarder = service.get_forwarder(address, external);

            log::info!("tcp socket accept: addr={address:?}, interface={local_addr:?}");

            // Disable the Nagle algorithm.
            // because to maintain real-time, any received data should be processed
            // as soon as possible.
            if let Err(e) = socket.set_nodelay(true) {
                log::error!("tcp socket set nodelay failed!: addr={address}, err={e}");
            }

            let id = Identifier {
                interface: external,
                source: address,
            };

            let session_manager = service.get_session_manager();
            tokio::spawn(async move {
                let mut buffer = ExchangeBuffer::default();

                'a: loop {
                    tokio::select! {
                        Ok(size) = socket.read(&mut buffer) => {
                            // When the received message is 0, it means that the socket
                            // has been closed.
                            if size == 0 {
                                break;
                            } else {
                                reporter.send(&id, &[Stats::ReceivedBytes(size)]);
                                buffer.advance(size);
                            }

                            // The minimum length of a stun message will not be less
                            // than 4.
                            if buffer.len() < 4 {
                                continue;
                            }

                            // Limit the maximum length of messages to 2048, this is to prevent buffer
                            // overflow attacks.
                            if buffer.len() > MAX_MESSAGE_SIZE * 3 {
                                break 'a;
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
                                    Ok(size) => {
                                        if size > MAX_MESSAGE_SIZE {
                                            log::warn!(
                                                "tcp message size too large: \
                                                size={size}, \
                                                max={MAX_MESSAGE_SIZE}, \
                                                addr={address:?}, \
                                                interface={local_addr:?}"
                                            );

                                            break 'a;
                                        }

                                        if size > buffer.len() {
                                            break;
                                        }

                                        reporter.send(&id, &[Stats::ReceivedPkts(1)]);

                                        size
                                    }
                                };

                                let chunk = buffer.split(size);
                                if let ForwardResult::Outbound(outbound) = forwarder.forward(chunk, address)
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

                                    if let Some(endpoint) = target.endpoint {
                                        exchanger.send(
                                            &endpoint,
                                            ty,
                                            target.relay.as_ref().unwrap_or_else(|| &address),
                                            bytes,
                                        );
                                    } else {
                                        if socket.write_all(bytes).await.is_err() {
                                            break 'a;
                                        }

                                        reporter.send(
                                            &id,
                                            &[Stats::SendBytes(bytes.len()), Stats::SendPkts(1)],
                                        );

                                        if let OutboundType::Message(method) = ty {
                                            if method.is_error() {
                                                reporter.send(&id, &[Stats::ErrorPkts(1)]);
                                            }
                                        }
                                    }
                                } else {
                                    break 'a;
                                }
                            }
                        }
                        Some((bytes, method, _)) = receiver.recv() => {
                            if socket.write_all(bytes.as_slice()).await.is_err() {
                                break;
                            } else {
                                reporter.send(&id, &[Stats::SendBytes(bytes.len()), Stats::SendPkts(1)]);
                            }

                            // The channel data needs to be aligned in multiples of 4 in
                            // tcp. If the channel data is forwarded to tcp, the alignment
                            // bit needs to be filled, because if the channel data comes
                            // from udp, it is not guaranteed to be aligned and needs to be
                            // checked.
                            if method == OutboundType::ChannelData {
                                let pad = bytes.len() % 4;
                                if pad > 0 && socket.write_all(&[0u8; 8][..(4 - pad)]).await.is_err() {
                                    break;
                                }
                            }
                        }
                        else => {
                            break;
                        }
                    }
                }

                // When the tcp connection is closed, the procedure to close the session is
                // process directly once, avoiding the connection being disconnected
                // directly without going through the closing
                // process.
                session_manager.refresh(&id, 0);

                exchanger.remove(&address);

                log::info!("tcp socket disconnect: addr={address:?}, interface={local_addr:?}");
            });
        }

        log::error!("tcp server close: interface={local_addr:?}");
    });

    log::info!("turn server listening: listen={listen}, external={external}, transport=TCP");

    Ok(())
}
