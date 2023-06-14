use turn_rs::Processor;
use bytes::{
    Bytes,
    BytesMut,
};

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};

use tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::TcpListener,
    sync::{
        mpsc::{
            channel,
            Receiver,
            Sender,
        },
        Mutex,
    },
};

pub struct Router {
    senders: Mutex<HashMap<SocketAddr, Sender<Bytes>>>,
}

impl Router {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            senders: Default::default(),
        })
    }

    pub async fn get(self: &Arc<Self>, addr: SocketAddr) -> Receiver<Bytes> {
        let (sender, mut receiver) = channel(10);

        {
            self.senders.lock().await.insert(addr, sender);
        }

        let this = self.clone();
        let (writer, reader) = channel(10);
        tokio::spawn(async move {
            while let Some(bytes) = receiver.recv().await {
                if writer.send(bytes).await.is_err() {
                    this.remove(&addr).await;
                    break;
                }
            }
        });

        reader
    }

    pub async fn send(&self, addr: &SocketAddr, data: &[u8]) {
        let mut is_err = false;

        {
            if let Some(sender) = self.senders.lock().await.get(addr) {
                if sender.send(Bytes::copy_from_slice(data)).await.is_err() {
                    is_err = true;
                }
            }
        }

        if is_err {
            self.remove(addr).await;
        }
    }

    async fn remove(&self, addr: &SocketAddr) {
        if let Some(sender) = self.senders.lock().await.remove(addr) {
            drop(sender)
        }
    }
}

pub async fn processer<T>(handle: T, listen: TcpListener, router: Arc<Router>)
where
    T: Fn() -> Processor,
{
    while let Ok((mut socket, addr)) = listen.accept().await {
        let router = router.clone();
        let mut processor = handle();
        tokio::spawn(async move {
            let (mut reader, mut writer) = socket.split();
            let mut receiver = router.get(addr.clone()).await;
            let mut buf = BytesMut::with_capacity(4096);

            tokio::select! {
                Ok(size) = reader.read_buf(&mut buf) => {
                    if let Ok(Some((data, addr))) = processor.process(&buf[..size], addr).await {
                        router.send(addr.as_ref(), data).await;
                    }
                }
                Some(bytes) = receiver.recv() => {
                    if writer.write_all(&bytes).await.is_err() {
                        return;
                    }
                }
            }
        });
    }
}
