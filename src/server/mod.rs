pub mod buffer;
pub mod exchanger;

pub mod transports {
    #[cfg(feature = "udp")]
    pub mod udp;

    #[cfg(feature = "tcp")]
    pub mod tcp;

    #[cfg(feature = "ssl")]
    pub mod tls;
}

use self::exchanger::Exchanger;

#[allow(unused)]
use crate::{
    config::{Config, Interface, Transport},
    statistics::Statistics,
};

use std::net::SocketAddr;

use codec::message::methods::Method;
use service::{Service, ServiceHandler};

pub const MAX_MESSAGE_SIZE: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboundType {
    Message(Method),
    ChannelData,
}

#[allow(unused)]
pub struct TransportOptions<'a, T> {
    config: &'a Config,
    listen: SocketAddr,
    external: SocketAddr,
    service: Service<T>,
    exchanger: Exchanger,
    statistics: Statistics,
}

/// start turn server.
///
/// create a specified number of threads,
/// each thread processes udp data separately.
pub async fn start<T>(
    config: &Config,
    statistics: &Statistics,
    service: &Service<T>,
) -> anyhow::Result<()>
where
    T: Clone + ServiceHandler + 'static,
{
    let exchanger = Exchanger::default();

    #[allow(unused)]
    for Interface {
        transport,
        external,
        listen,
        ssl,
    } in config.turn.interfaces.iter().cloned()
    {
        let options = TransportOptions {
            statistics: statistics.clone(),
            exchanger: exchanger.clone(),
            service: service.clone(),
            external,
            config,
            listen,
        };

        match transport {
            #[cfg(feature = "udp")]
            Transport::Udp => transports::udp::listener(options).await?,
            #[cfg(all(feature = "tcp", not(feature = "ssl")))]
            Transport::Tcp => transports::tcp::listener(options).await?,
            #[cfg(all(feature = "tcp", feature = "ssl"))]
            Transport::Tcp => {
                if let Some(ssl) = ssl {
                    transports::tls::listener(options, ssl).await?
                } else {
                    transports::tcp::listener(options).await?
                }
            }
            #[allow(unreachable_patterns)]
            _ => (),
        };
    }

    Ok(())
}
