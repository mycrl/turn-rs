use turn_rs::Processor;
use anyhow::Result;
use std::{
    collections::HashMap,
    net::SocketAddr,
    net::IpAddr,
    sync::Arc,
};

use tokio::io::{
    AsyncReadExt,
    AsyncWriteExt,
};

use tokio::sync::mpsc::{
    channel,
    Receiver,
    Sender,
};

use tokio::{
    net::TcpListener,
    net::UdpSocket,
    sync::Mutex,
};

use crate::monitor::{
    MonitorSender,
    Payload,
};

/// udp socket process thread.
///
/// read the data packet from the UDP socket and hand
/// it to the proto for processing, and send the processed
/// data packet to the specified address.
struct UdpProxy {
    v4: UdpSocket,
    v6: UdpSocket,
}

impl UdpProxy {
    async fn new() -> Result<Self> {
        Ok(Self {
            v4: UdpSocket::bind("0.0.0.0:0").await?,
            v6: UdpSocket::bind("[::]:0").await?,
        })
    }

    async fn send(&self, data: &[u8], addr: &SocketAddr) {
        match addr.ip() {
            IpAddr::V4(_) => self.v4.send_to(data, addr).await,
            IpAddr::V6(_) => self.v6.send_to(data, addr).await,
        }
        .expect("there is an error in the udp proxy in tcp!");
    }
}

pub struct Router {
    senders: Mutex<HashMap<SocketAddr, Sender<&'static [u8]>>>,
    udp: UdpProxy,
}

impl Router {
    pub async fn new() -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            senders: Default::default(),
            udp: UdpProxy::new().await?,
        }))
    }

    /// udp socket process thread.
    ///
    /// read the data packet from the UDP socket and hand
    /// it to the proto for processing, and send the processed
    /// data packet to the specified address.
    async fn get(
        self: &Arc<Self>,
        addr: SocketAddr,
    ) -> Receiver<&'static [u8]> {
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

    /// udp socket process thread.
    ///
    /// read the data packet from the UDP socket and hand
    /// it to the proto for processing, and send the processed
    /// data packet to the specified address.
    async fn send(&self, addr: &SocketAddr, data: &[u8]) {
        let mut is_err = false;

        {
            // udp socket process thread.
            //
            // read the data packet from the UDP socket and hand
            // it to the proto for processing, and send the processed
            // data packet to the specified address.
            if let Some(sender) = self.senders.lock().await.get(addr) {
                if sender
                    .send(unsafe { std::mem::transmute(data) })
                    .await
                    .is_err()
                {
                    is_err = true;
                }
            } else {
                self.udp.send(data, addr).await;
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

/// udp socket process thread.
///
/// read the data packet from the UDP socket and hand
/// it to the proto for processing, and send the processed
/// data packet to the specified address.
pub async fn processer<T>(
    handle: T,
    sender: Arc<MonitorSender>,
    router: Arc<Router>,
    listen: TcpListener,
) where
    T: Fn() -> Processor,
{
    let local_addr = listen
        .local_addr()
        .expect("get tcp listener local addr failed!");

    // udp socket process thread.
    //
    // read the data packet from the UDP socket and hand
    // it to the proto for processing, and send the processed
    // data packet to the specified address.
    while let Ok((mut socket, addr)) = listen.accept().await {
        let router = router.clone();
        let sender = sender.clone();
        let mut processor = handle();

        log::info!(
            "tcp socket accept: addr={:?}, interface={:?}",
            addr,
            local_addr,
        );

        tokio::spawn(async move {
            let (mut reader, mut writer) = socket.split();
            let mut receiver = router.get(addr.clone()).await;
            let mut buf = [0u8; 4096];

            loop {
                tokio::select! {
                    Ok(size) = reader.read(&mut buf) => {
                        if size > 0 {
                            sender.send(Payload::Receive);
                            log::trace!(
                                "tcp socket receive: size={}, addr={:?}, interface={:?}",
                                size,
                                addr,
                                local_addr,
                            );

                            // udp socket process thread.
                            //
                            // read the data packet from the UDP socket and hand
                            // it to the proto for processing, and send the processed
                            // data packet to the specified address.
                            if let Ok(Some((data, addr))) = processor.process(&buf[..size], addr).await {
                                router.send(addr.as_ref(), data).await;
                                sender.send(Payload::Send);
                            } else {
                                sender.send(Payload::Failed);
                            }
                        } else {
                            break;
                        }
                    }
                    Some(bytes) = receiver.recv() => {
                        if writer.write_all(bytes).await.is_err() {
                            break;
                        }

                        log::trace!(
                            "tcp socket relay: size={}, addr={:?}",
                            bytes.len(),
                            addr,
                        );
                    }
                }
            }

            log::info!(
                "tcp socket disconnect: addr={:?}, interface={:?}",
                addr,
                local_addr,
            );
        });
    }
}
