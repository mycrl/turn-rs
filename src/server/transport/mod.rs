pub mod tcp;
pub mod udp;

use std::{net::SocketAddr, task::Poll, time::Duration};

use anyhow::Result;
use bytes::Bytes;
use tokio::time::interval;

use crate::{
    Service,
    config::Ssl,
    server::Exchanger,
    service::{Transport, session::Identifier},
    statistics::{Statistics, Stats},
};

pub const MAX_MESSAGE_SIZE: usize = 4096;

pub trait Socket: Send + 'static {
    fn read(&mut self) -> impl Future<Output = Result<Bytes>> + Send;
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

pub trait Server: Sized + Send {
    type Socket: Socket;

    /// Bind the server to the specified address.
    fn bind(options: &ServerOptions) -> impl Future<Output = Result<Self>> + Send;

    /// Accept a new connection.
    fn accept(&mut self) -> impl Future<Output = Result<Poll<(Self::Socket, SocketAddr)>>> + Send;

    /// Get the local address of the listener.
    fn local_addr(&self) -> Result<SocketAddr>;

    /// Start the server.
    fn start(
        options: ServerOptions,
        service: Service,
        statistics: Statistics,
        exchanger: Exchanger,
    ) -> impl Future<Output = Result<()>> + Send {
        let transport = options.transport;
        let idle_timeout = options.idle_timeout as u64;

        async move {
            let mut listener = Self::bind(&options).await?;
            let local_addr = listener.local_addr()?;

            log::info!(
                "server listening: listen={}, external={}, local addr={local_addr}, transport={transport:?}",
                options.listen,
                options.external,
            );

            while let Ok(poll) = listener.accept().await {
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
                let mut receiver = exchanger.get_receiver(id);
                let reporter = statistics.get_reporter(transport);

                let service = service.clone();
                let exchanger = exchanger.clone();

                tokio::spawn(async move {
                    let mut interval = interval(Duration::from_secs(1));
                    let mut read_delay = 0;

                    loop {
                        tokio::select! {
                            Ok(buffer) = socket.read() => {
                                read_delay = 0;

                                if let Ok(Some(res)) = router.route(&buffer).await
                                {

                                    if let Some(relay) = res.relay {
                                        exchanger.send(&relay, res.bytes);

                                        // The channel data needs to be aligned in multiples of 4 in
                                        // tcp. If the channel data is forwarded to tcp, the alignment
                                        // bit needs to be filled, because if the channel data comes
                                        // from udp, it is not guaranteed to be aligned and needs to be
                                        // checked.
                                        if relay.transport == Transport::Tcp && res.method.is_none() {
                                            let pad = res.bytes.len() % 4;
                                            if pad > 0 {
                                                exchanger.send(&relay, &[0u8; 8][..(4 - pad)]);
                                            }
                                        }
                                    } else {
                                        if socket.write(res.bytes).await.is_err() {
                                            break;
                                        }

                                        reporter.send(
                                            &id,
                                            &[Stats::SendBytes(res.bytes.len()), Stats::SendPkts(1)],
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

                    exchanger.remove(&id);

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
