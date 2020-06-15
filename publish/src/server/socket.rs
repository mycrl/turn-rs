use super::Tx;
use crate::codec::{Codec, Packet};
use bytes::BytesMut;
use std::marker::Unpin;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::io::{Error, ErrorKind};
use tokio::net::TcpStream;
use transport::Flag;

/// TcpSocket instance
///
/// Read and write TcpSocket and return data through channel.
/// The returned data is a Udp data packet. In order to adapt to MTU,
/// the subcontracting has been completed.
pub struct Socket<T> {
    stream: TcpStream,
    forward: Tx,
    codec: T,
}

impl<T: Default + Codec + Unpin> Socket<T> {
    /// Create a TcpSocket instance
    ///
    /// To create an instance, you need to specify a `Codec` as the data codec.
    /// `Codec` processes Tcp data and asks for the returned Tcp data and Udp packet.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    /// use tokio::net::TcpListener;
    /// use socket::Socket;
    /// use rtmp::Rtmp;
    ///
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let addr = "0.0.0.0:1935".parse().unwrap();
    ///     let mut listener = TcpListener::bind(&addr).await?;
    ///     
    ///     loop {
    ///         let (stream, _) = listener.accept().await?;
    ///         tokio::spawn(Socket::<Rtmp>::new(stream));
    ///     }
    /// }
    /// ```
    pub fn new(stream: TcpStream, forward: Tx) -> Self {
        Self {
            stream,
            forward,
            codec: T::default(),
        }
    }

    /// Push messages to channel
    ///
    /// Push the chunk package to the channel.
    /// The other end needs to send data to the remote TcpServer.
    #[rustfmt::skip]
    fn push(&mut self, data: BytesMut, flag: Flag) -> Result<(), Error> {
        match self.forward.send((flag, data)) {
            Err(e) => Err(Error::new(ErrorKind::BrokenPipe, e.to_string())),
            Ok(_) => Ok(())
        }
    }

    /// Try to process TcpSocket data
    ///
    /// Use `Codec` to handle TcpSocket data,
    /// Write the returned data to TcpSocket or UdpSocket correctly.
    pub async fn process(&mut self) -> Result<(), Error> {
        let mut receiver = [0u8; 2048];
        let size = self.stream.read(&mut receiver).await?;
        let mut chunk = BytesMut::from(&receiver[0..size]);
        for packet in self.codec.parse(&mut chunk) {
            match packet {
                Packet::Peer(data) => self.stream.write_all(&data).await?,
                Packet::Core(data, flag) => self.push(data, flag)?,
            }
        }

        // Refresh the TcpSocket buffer. In order to increase efficiency,
        // all the returned data of the current task will be written and
        // then refreshed in a unified manner to avoid unnecessary frequent operations.
        self.stream.flush().await?;
        Ok(())
    }
}
