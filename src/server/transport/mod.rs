pub mod tcp;
pub mod udp;

use std::{net::SocketAddr, time::Duration};

use anyhow::Result;
use bytes::Bytes;

use tokio::time::interval;

use crate::{
    Service,
    config::Ssl,
    server::{Exchanger, PayloadType},
    service::{routing::Response, session::Identifier},
    statistics::{Statistics, Stats},
};

pub const MAX_MESSAGE_SIZE: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Udp,
    Tcp,
}

pub trait Socket: Send + 'static {
    fn read(&mut self) -> impl Future<Output = Option<Bytes>> + Send;
    fn write(&mut self, buffer: &[u8]) -> impl Future<Output = Result<()>> + Send;
    fn close(&mut self) -> impl Future<Output = ()> + Send;
}

#[allow(unused)]
pub struct ListenOptions {
    pub transport: Transport,
    pub idle_timeout: u32,
    pub listen: SocketAddr,
    pub external: SocketAddr,
    pub ssl: Option<Ssl>,
    pub mtu: usize,
}

pub trait Listener: Sized + Send {
    type Socket: Socket;

    fn bind(options: &ListenOptions) -> impl Future<Output = Result<Self>> + Send;
    fn accept(&mut self) -> impl Future<Output = Option<(Self::Socket, SocketAddr)>> + Send;
    fn local_addr(&self) -> Result<SocketAddr>;

    fn start(
        options: ListenOptions,
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

            while let Some((mut socket, address)) = listener.accept().await {
                let id = Identifier {
                    interface: options.external,
                    source: address,
                };

                let mut receiver = exchanger.get_receiver(address);
                let mut router = service.make_router(address, options.external);
                let reporter = statistics.get_reporter();

                let service = service.clone();
                let exchanger = exchanger.clone();

                tokio::spawn(async move {
                    let mut interval = interval(Duration::from_secs(1));
                    let mut read_delay = 0;

                    loop {
                        tokio::select! {
                            Some(buffer) = socket.read() => {
                                read_delay = 0;

                                if let Ok(res) = router.route(&buffer, address).await
                                {
                                    let (ty, bytes, target) = if let Some(it) = res {
                                        (
                                            it.method.map(PayloadType::Message).unwrap_or(PayloadType::ChannelData),
                                            it.bytes,
                                            it.target,
                                        )
                                    } else {
                                        continue;
                                    };

                                    if let Some(endpoint) = target.endpoint {
                                        exchanger.send(&endpoint, ty, Bytes::copy_from_slice(bytes));
                                    } else {
                                        if socket.write(bytes).await.is_err() {
                                            break;
                                        }

                                        reporter.send(
                                            &id,
                                            &[Stats::SendBytes(bytes.len()), Stats::SendPkts(1)],
                                        );

                                        if let PayloadType::Message(method) = ty && method.is_error() {
                                            reporter.send(&id, &[Stats::ErrorPkts(1)]);
                                        }
                                    }
                                }
                            }
                            Some((bytes, method)) = receiver.recv() => {
                                if socket.write(&bytes).await.is_err() {
                                    break;
                                } else {
                                    reporter.send(&id, &[Stats::SendBytes(bytes.len()), Stats::SendPkts(1)]);
                                }

                                // The channel data needs to be aligned in multiples of 4 in
                                // tcp. If the channel data is forwarded to tcp, the alignment
                                // bit needs to be filled, because if the channel data comes
                                // from udp, it is not guaranteed to be aligned and needs to be
                                // checked.
                                if transport == Transport::Tcp && method == PayloadType::ChannelData {
                                    let pad = bytes.len() % 4;
                                    if pad > 0 && socket.write(&[0u8; 8][..(4 - pad)]).await.is_err() {
                                        break;
                                    }
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

                    exchanger.remove(&address);

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
