use std::{net::SocketAddr, task::Poll};

#[cfg(feature = "ssl")]
use std::sync::Arc;

use anyhow::{Result, anyhow};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
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

use crate::{
    codec::Decoder,
    server::{
        memory_pool::{Buffer, MemoryPool},
        provider::{ProviderServer, ProviderStream, ServerOptions},
    },
};

pub enum MaybeSslStream {
    Base(TcpStream),
    #[cfg(feature = "ssl")]
    Ssl(TlsStream<TcpStream>),
}

impl ProviderStream for MaybeSslStream {
    async fn read(&mut self) -> Result<Buffer> {
        let mut buffer = MemoryPool::acquire();

        unsafe {
            buffer.set_len(4);
        }

        let size = {
            if match self {
                #[cfg(feature = "ssl")]
                Self::Ssl(stream) => stream.read_exact(&mut buffer[..4]).await?,
                Self::Base(stream) => stream.read_exact(&mut buffer[..4]).await?,
            } < 4
            {
                return Err(anyhow!("failed to read the first 4 bytes of the message"));
            }

            Decoder::message_size(&buffer[..4], true)?
        };

        // The buffer is resized to the actual size of the message, which is determined by the first 4 bytes of the message.
        if size > MemoryPool::MAX_MESSAGE_SIZE {
            return Err(anyhow!(
                "message size {} exceeds the maximum allowed size",
                size
            ));
        }

        // SAFETY: The buffer is initialized with zeroes and the length is set to
        // the actual size of the message, which is determined by the first 4
        // bytes of the message.
        //
        // The buffer is not used until it is fully initialized, so it is safe to
        // set the length after reading the message.
        unsafe {
            buffer.set_len(size);
        }

        // Read the rest of the message based on the size determined by the first 4 bytes.
        if match self {
            #[cfg(feature = "ssl")]
            Self::Ssl(stream) => stream.read_exact(&mut buffer[4..size]).await?,
            Self::Base(stream) => stream.read_exact(&mut buffer[4..size]).await?,
        } < size - 4
        {
            return Err(anyhow!("failed to read the full message"));
        }

        Ok(buffer)
    }

    async fn write(&mut self, buffer: &[u8]) -> Result<()> {
        match self {
            #[cfg(feature = "ssl")]
            Self::Ssl(stream) => stream.write_all(buffer).await?,
            Self::Base(stream) => stream.write_all(buffer).await?,
        }

        Ok(())
    }

    async fn close(&mut self) {
        match self {
            #[cfg(feature = "ssl")]
            Self::Ssl(stream) => {
                let _ = stream.shutdown().await;
            }
            Self::Base(stream) => {
                let _ = stream.shutdown().await;
            }
        }
    }
}

pub struct TcpServer {
    listener: TcpListener,
    local_addr: SocketAddr,
    #[cfg(feature = "ssl")]
    acceptor: Option<TlsAcceptor>,
}

impl ProviderServer for TcpServer {
    type Stream = MaybeSslStream;

    async fn bind(options: &ServerOptions) -> Result<Self> {
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

        let listener = TcpListener::bind(options.listen).await?;
        let local_addr = listener.local_addr()?;

        Ok(Self {
            listener,
            local_addr,
            #[cfg(feature = "ssl")]
            acceptor,
        })
    }

    async fn accept(&mut self) -> Result<Poll<(Self::Stream, SocketAddr)>> {
        let (socket, addr) = self.listener.accept().await?;

        // Disable the Nagle algorithm.
        // because to maintain real-time, any received data should be processed
        // as soon as possible.
        if let Err(e) = socket.set_nodelay(true) {
            log::warn!("tls socket set nodelay failed!: addr={addr}, err={e}");
        }

        #[cfg(feature = "ssl")]
        if let Some(ref acceptor) = self.acceptor {
            return Ok(if let Ok(socket) = acceptor.accept(socket).await {
                Poll::Ready((MaybeSslStream::Ssl(socket), addr))
            } else {
                Poll::Pending
            });
        }

        Ok(Poll::Ready((MaybeSslStream::Base(socket), addr)))
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }
}
