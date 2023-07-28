pub mod transport;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use self::transport::{OrderTransport, Transport, TransportAddr};

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct ProxyStateNotifyNode {
    pub external: SocketAddr,
    pub addr: SocketAddr,
    pub online: bool,
    pub index: u8,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Request {
    ProxyStateNotify(Vec<ProxyStateNotifyNode>),
}

impl TryFrom<&[u8]> for Request {
    type Error = anyhow::Error;

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(rmp_serde::from_slice(value)?)
    }
}

impl From<Request> for Vec<u8> {
    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    fn from(val: Request) -> Self {
        rmp_serde::to_vec(&val).expect("serde to json string failed!")
    }
}

pub trait RpcObserver: Send + Sync {
    fn on(&mut self, req: Request);
    fn on_relay(&mut self, payload: &[u8]);
}

pub struct Rpc {
    sender: mpsc::Sender<(Request, u8)>,
    transport: Transport,
}

impl Rpc {
    pub async fn new<T: RpcObserver + 'static>(
        addr: TransportAddr,
        mut observer: T,
    ) -> Result<Arc<Self>> {
        let (sender, mut receiver) = mpsc::channel::<(Request, u8)>(5);

        let mut order_transport = OrderTransport::new(addr).await?;
        let transport = Transport::new(addr).await?;
        let transport_ = transport.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];

            loop {
                tokio::select! {
                    Some((buf, _)) = order_transport.recv() => {
                        if let Ok(req) = Request::try_from(buf.as_ref()) {
                            observer.on(req);
                        }
                    }
                    Ok(ret) = transport_.recv(&mut buf) => {
                        if let Some((buf, _)) = ret {
                            observer.on_relay(buf);
                        }
                    }
                    Some((req, to)) = receiver.recv() => {
                        let buf: Vec<u8> = req.into();
                        if order_transport.send(&buf, to).await.is_err() {
                            break;
                        }
                    }
                    else => {
                        break;
                    }
                }
            }
        });

        Ok(Arc::new(Self { transport, sender }))
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub async fn send_with_order(&self, req: Request, to: u8) -> Result<()> {
        self.sender.send((req, to)).await?;
        Ok(())
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    pub async fn send(&self, payload: &[u8], to: u8) -> Result<()> {
        self.transport.send(&payload, to).await?;
        Ok(())
    }
}