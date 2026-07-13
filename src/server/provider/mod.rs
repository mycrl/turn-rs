pub mod tcp;
pub mod udp;

use std::{net::SocketAddr, ops::DerefMut, task::Poll, time::Duration};

use anyhow::Result;
use tokio::{
    sync::{mpsc::UnboundedSender, watch},
    time::interval,
};

use crate::{
    codec::{
        channel_data::ChannelData,
        message::{
            MessageEncoder,
            attributes::{Data, XorPeerAddress},
            methods::DATA_INDICATION,
        },
    },
    config::Ssl,
    server::{Switch, buffer::Buffer},
    service::{Service, ServiceHandler, Transport, routing::RelayTarget, session::Identifier},
    statistics::{Statistics, Stats},
};

pub trait ProviderStream: Send + 'static {
    fn read(&mut self) -> impl Future<Output = Result<Buffer>> + Send;
    fn write(&mut self, buffer: &[u8]) -> impl Future<Output = Result<()>> + Send;
    fn close(&mut self) -> impl Future<Output = ()> + Send;
}

#[allow(unused)]
pub struct ServerOptions {
    pub transport: Transport,
    pub idle_timeout: u32,
    pub listen: SocketAddr,
    pub external: SocketAddr,
    pub ssl: Option<Ssl>,
    pub mtu: usize,
}

pub trait ProviderServer: Sized + Send {
    type Stream: ProviderStream;

    /// Bind the server to the specified address.
    fn bind(
        options: &ServerOptions,
        shutdown: watch::Receiver<bool>,
    ) -> impl Future<Output = Result<Self>> + Send;

    /// Accept a new connection.
    fn accept(&mut self) -> impl Future<Output = Result<Poll<(Self::Stream, SocketAddr)>>> + Send;

    /// Get the local address of the listener.
    fn local_addr(&self) -> Result<SocketAddr>;

    /// Start the server.
    fn start<T>(
        options: ServerOptions,
        service: Service<T>,
        statistics: Statistics,
        switch: Switch,
        shutdown: watch::Receiver<bool>,
        startup: UnboundedSender<Result<()>>,
    ) -> impl Future<Output = Result<()>> + Send
    where
        T: ServiceHandler + Clone,
    {
        let transport = options.transport;
        let idle_timeout = options.idle_timeout as u64;

        async move {
            let mut listener = match Self::bind(&options, shutdown.clone()).await {
                Ok(listener) => listener,
                Err(error) => {
                    let _ = startup.send(Err(anyhow::anyhow!(error.to_string())));
                    return Err(error);
                }
            };
            let local_addr = match listener.local_addr() {
                Ok(local_addr) => local_addr,
                Err(error) => {
                    let _ = startup.send(Err(anyhow::anyhow!(error.to_string())));
                    return Err(error);
                }
            };
            let _ = startup.send(Ok(()));
            let mut shutdown = shutdown;

            log::info!(
                "server listening: listen={}, external={}, local addr={local_addr}, transport={transport:?}",
                options.listen,
                options.external,
            );

            loop {
                let poll = tokio::select! {
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() {
                            break;
                        }
                        continue;
                    }
                    result = listener.accept() => result?,
                };
                let Poll::Ready((mut socket, address)) = poll else {
                    continue;
                };

                let id = Identifier {
                    source: address,
                    interface: local_addr,
                    external: options.external,
                    transport: options.transport,
                };

                let mut router = service.make_router(id);
                let mut receiver = switch.get_receiver(id);
                let reporter = statistics.get_reporter(transport);

                let service = service.clone();
                let switch = switch.clone();
                let mut session_shutdown = shutdown.clone();

                tokio::spawn(async move {
                    let mut interval = interval(Duration::from_secs(1));
                    let mut read_delay = 0;

                    loop {
                        let mut response_buffer = Buffer::new();

                        tokio::select! {
                            changed = session_shutdown.changed() => {
                                if changed.is_err() || *session_shutdown.borrow() {
                                    break;
                                }
                            }
                            Ok(buffer) = socket.read() => {
                                read_delay = 0;

                                if let Ok(Some(res)) = router.route(&buffer, &mut response_buffer).await
                                {

                                    if let Some(relay_socket) = res.allocation {
                                            let relay_service = service.clone();
                                            let relay_switch = switch.clone();
                                            let relay_id = id;
                                            let relay_shutdown = session_shutdown.clone();
                                            tokio::spawn(async move {
                                                let mut shutdown = relay_shutdown;
                                                let mut cleanup = tokio::time::interval(Duration::from_secs(1));
                                                loop {
                                                    let mut packet = Buffer::new();
                                                    tokio::select! {
                                                        changed = shutdown.changed() => {
                                                            if changed.is_err() || *shutdown.borrow() {
                                                                break;
                                                            }
                                                        }
                                                        _ = cleanup.tick() => {
                                                            if relay_service.get_session_manager().relay_socket(&relay_id).is_none() {
                                                                break;
                                                            }
                                                        }
                                                        received = relay_socket.recv_buf_from(packet.deref_mut()) => {
                                                            let Ok((_size, peer)) = received else {
                                                                break;
                                                            };
                                                            let Some(inbound) = relay_service.get_session_manager().relay_from_peer(&relay_id, peer) else {
                                                                continue;
                                                            };
                                                            let mut outbound = Buffer::new();
                                                            if let Some(channel) = inbound.channel {
                                                                ChannelData::new(channel, &packet).encode(&mut outbound);
                                                            } else {
                                                                let mut message = MessageEncoder::new(DATA_INDICATION, &[0; 12], &mut outbound);
                                                                message.append::<XorPeerAddress>(peer);
                                                                message.append::<Data>(&packet);
                                                                if message.flush(None).is_err() {
                                                                    continue;
                                                                }
                                                            }
                                                            relay_switch.send(&relay_id, outbound);
                                                        }
                                                    }
                                                }
                                            });
                                        }

                                        if let Some(RelayTarget::Peer { socket: relay_socket, peer }) = res.relay {
                                            let _ = relay_socket.send_to(&response_buffer, peer).await;
                                        } else {
                                            if socket.write(&response_buffer).await.is_err() {
                                                break;
                                            }

                                            reporter.send(
                                                &id,
                                                &[Stats::SendBytes(response_buffer.len()), Stats::SendPkts(1)],
                                            );

                                            if let Some(method) = res.method && method.is_error() {
                                                reporter.send(&id, &[Stats::ErrorPkts(1)]);
                                            }
                                        }
                                }
                            }
                            Some(bytes) = receiver.recv() => {
                                if socket.write(&bytes).await.is_err() {
                                    break;
                                } else {
                                    reporter.send(&id, &[Stats::SendBytes(bytes.len()), Stats::SendPkts(1)]);
                                }
                            }
                            _ = interval.tick() => {
                                read_delay += 1;

                                if read_delay >= idle_timeout {
                                    break;
                                }
                            }
                            else => {
                                break;
                            }
                        }
                    }

                    // close the socket
                    socket.close().await;

                    // When the socket connection is closed, the procedure to close the session is
                    // process directly once, avoiding the connection being disconnected
                    // directly without going through the closing
                    // process.
                    service.get_session_manager().refresh(&id, 0);

                    switch.remove(&id);

                    log::info!(
                        "socket disconnect: addr={address:?}, interface={local_addr:?}, transport={transport:?}"
                    );
                });
            }

            log::error!("server shutdown: interface={local_addr:?}, transport={transport:?}");

            Ok(())
        }
    }
}
