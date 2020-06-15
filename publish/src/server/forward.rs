use super::Rx;
use std::{io::Error, net::SocketAddr};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use transport::Transport;

/// Data advancement
///
/// Push the event and data of the instance
/// to other business backends through TCPSocket.
///
/// TODO: 单路TCP负载能力有限，
/// 计划使用多路合并来提高传输能力;
pub struct Forward {
    stream: TcpStream,
    receiver: Rx,
}

impl Forward {
    /// Create an example of data advancement
    ///
    /// Specify a remote address and data pipeline bus
    /// to create an instance, which is responsible for
    /// serializing the data into tcp data stream and
    /// pushing it to other business backends.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use forward::Forward;
    /// use tokio::sync::mpsc;
    ///
    /// let addr = "127.0.0.1:1936".parse().unwrap();
    /// let (_, receiver) = mpsc::unbounded_channel();
    /// let forward = Forward::new(addr, receiver).await?;
    /// tokio::spawn(forward);
    /// ```
    pub async fn new(addr: SocketAddr, receiver: Rx) -> Result<Self, Error> {
        Ok(Self {
            receiver,
            stream: TcpStream::connect(addr).await?,
        })
    }

    /// Handling pipeline messages
    ///
    /// Try to process the backlog message in the
    /// pipeline, and serialize it into tcp protocol
    /// packet through the data transfer module to
    /// send to tcpsocket.
    pub async fn process(&mut self) -> Result<(), Error> {
        if let Some((flag, data)) = self.receiver.recv().await {
            let buffer = Transport::encoder(data, flag);
            self.stream.write_all(&buffer).await?;
            self.stream.flush().await?;
        }

        Ok(())
    }
}
