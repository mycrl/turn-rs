use std::{net::SocketAddr, task::Poll};

use anyhow::{Result, anyhow};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{
    codec::Decoder,
    server::{
        buffer::Buffer,
        provider::{ProviderServer, ProviderStream, ServerOptions},
    },
};

impl ProviderStream for TcpStream {
    async fn read(&mut self) -> Result<Buffer> {
        let mut buffer = Buffer::new();

        unsafe {
            buffer.set_len(4);
        }

        let size = {
            if self.read_exact(&mut buffer[..4]).await? < 4 {
                return Err(anyhow!("failed to read the first 4 bytes of the message"));
            }

            Decoder::message_size(&buffer[..4], true)?
        };

        // The buffer is resized to the actual size of the message, which is determined by the first 4 bytes of the message.
        if size > Buffer::MAX_MESSAGE_SIZE {
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
        if self.read_exact(&mut buffer[4..size]).await? < size - 4 {
            return Err(anyhow!("failed to read the full message"));
        }

        Ok(buffer)
    }

    async fn write(&mut self, buffer: &[u8]) -> Result<()> {
        self.write_all(buffer).await?;

        Ok(())
    }

    async fn close(&mut self) {
        let _ = self.shutdown().await;
    }
}

pub struct TcpServer {
    listener: TcpListener,
    local_addr: SocketAddr,
}

impl ProviderServer for TcpServer {
    type Stream = TcpStream;

    async fn bind(options: &ServerOptions) -> Result<Self> {
        let listener = TcpListener::bind(options.listen).await?;
        let local_addr = listener.local_addr()?;

        Ok(Self {
            listener,
            local_addr,
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

        Ok(Poll::Ready((socket, addr)))
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }
}
