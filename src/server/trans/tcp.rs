use std::{io::Error, net::SocketAddr};

#[cfg(feature = "ssl")]
use std::sync::Arc;

use anyhow::Result;
use bytes::{Bytes, BytesMut};
use codec::Decoder;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::{TcpListener as TokioTcpListener, TcpStream},
    sync::mpsc::{Sender, UnboundedReceiver, channel, unbounded_channel},
};

#[cfg(feature = "ssl")]
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        ServerConfig,
        pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    },
    server::TlsStream,
};

use crate::server::trans::{ListenOptions, Listener, MAX_MESSAGE_SIZE, Socket};

enum MaybeSslStream {
    #[cfg(feature = "ssl")]
    Ssl(Box<TlsStream<TcpStream>>),
    Base(TcpStream),
}

impl MaybeSslStream {
    fn split(self) -> (Reader, Writer) {
        use tokio::io::split;

        match self {
            Self::Base(it) => {
                let (rx, tx) = split(it);

                (Reader::Base(rx), Writer::Base(tx))
            }
            #[cfg(feature = "ssl")]
            Self::Ssl(it) => {
                let (rx, tx) = split(it);

                (Reader::Ssl(rx), Writer::Ssl(tx))
            }
        }
    }
}

enum Reader {
    #[cfg(feature = "ssl")]
    Ssl(ReadHalf<Box<TlsStream<TcpStream>>>),
    Base(ReadHalf<TcpStream>),
}

impl Reader {
    async fn read_buf(&mut self, buffer: &mut BytesMut) -> Result<usize, Error> {
        match self {
            Self::Base(it) => it.read_buf(buffer).await,
            #[cfg(feature = "ssl")]
            Self::Ssl(it) => it.read_buf(buffer).await,
        }
    }
}

enum Writer {
    #[cfg(feature = "ssl")]
    Ssl(WriteHalf<Box<TlsStream<TcpStream>>>),
    Base(WriteHalf<TcpStream>),
}

impl Writer {
    async fn write_all(&mut self, buffer: &[u8]) -> Result<(), Error> {
        match self {
            Self::Base(it) => it.write_all(buffer).await,
            #[cfg(feature = "ssl")]
            Self::Ssl(it) => it.write_all(buffer).await,
        }
    }
}

pub struct TcpSocket {
    writer: Writer,
    receiver: UnboundedReceiver<Bytes>,
    close_signal_sender: Sender<()>,
}

impl TcpSocket {
    fn new(stream: MaybeSslStream, addr: SocketAddr) -> Self {
        let (close_signal_sender, mut close_signal_receiver) = channel::<()>(1);
        let (tx, receiver) = unbounded_channel::<Bytes>();
        let (mut reader, writer) = stream.split();

        tokio::spawn(async move {
            let mut buffer = BytesMut::new();

            'a: loop {
                tokio::select! {
                    Ok(size) = reader.read_buf(&mut buffer) => {
                        if size == 0 {
                            break;
                        }

                        // The minimum length of a stun message will not be less
                        // than 4.
                        if buffer.len() < 4 {
                            continue;
                        }

                        // Limit the maximum length of messages to 2048, this is to prevent buffer
                        // overflow attacks.
                        if buffer.len() > MAX_MESSAGE_SIZE * 3 {
                            break;
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
                                                addr={addr:?}"
                                        );

                                        break 'a;
                                    }

                                    if size > buffer.len() {
                                        break;
                                    }

                                    size
                                }
                            };

                            if tx.send(buffer.split_to(size).freeze()).is_err() {
                                break 'a;
                            }
                        }
                    }
                    _ = close_signal_receiver.recv() => {
                        break;
                    }
                    else => {
                        break;
                    }
                }
            }
        });

        Self {
            close_signal_sender,
            writer,
            receiver,
        }
    }
}

impl Socket for TcpSocket {
    async fn read(&mut self) -> Option<Bytes> {
        self.receiver.recv().await
    }

    async fn write(&mut self, buffer: &[u8]) -> Result<()> {
        Ok(self.writer.write_all(buffer).await?)
    }

    async fn close(&mut self) {
        self.receiver.close();

        let _ = self.close_signal_sender.send(()).await;
    }
}

pub struct TcpServer {
    socket_receiver: UnboundedReceiver<(TcpSocket, SocketAddr)>,
    local_addr: SocketAddr,
}

impl Listener for TcpServer {
    type Socket = TcpSocket;

    async fn bind(options: &ListenOptions) -> Result<Self> {
        #[cfg(feature = "ssl")]
        let acceptor = if let Some(ssl) = &options.ssl {
            Some(TlsAcceptor::from(Arc::new(
                ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(
                        CertificateDer::pem_file_iter(ssl.certificate_chain.clone())?
                            .collect::<Result<Vec<_>, _>>()?,
                        PrivateKeyDer::from_pem_file(ssl.private_key.clone())?,
                    )?,
            )))
        } else {
            None
        };

        let listener = TokioTcpListener::bind(options.listen).await?;
        let local_addr = listener.local_addr()?;

        let (tx, socket_receiver) = unbounded_channel::<(TcpSocket, SocketAddr)>();
        tokio::spawn(async move {
            while let Ok((socket, addr)) = listener.accept().await {
                // Disable the Nagle algorithm.
                // because to maintain real-time, any received data should be processed
                // as soon as possible.
                if let Err(e) = socket.set_nodelay(true) {
                    log::warn!("tls socket set nodelay failed!: addr={addr}, err={e}");
                }

                #[cfg(feature = "ssl")]
                if let Some(acceptor) = acceptor.clone() {
                    let tx = tx.clone();

                    tokio::spawn(async move {
                        if let Ok(socket) = acceptor.accept(socket).await {
                            let _ = tx.send((
                                TcpSocket::new(MaybeSslStream::Ssl(socket.into()), addr),
                                addr,
                            ));
                        };
                    });

                    continue;
                }

                if tx
                    .send((TcpSocket::new(MaybeSslStream::Base(socket), addr), addr))
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok(Self {
            socket_receiver,
            local_addr,
        })
    }

    async fn accept(&mut self) -> Option<(Self::Socket, SocketAddr)> {
        self.socket_receiver.recv().await
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }
}
