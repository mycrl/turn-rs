pub mod transport;

use std::{
    net::SocketAddr,
    sync::Arc,
};

use anyhow::Result;
use tokio::sync::mpsc;
use serde::{
    Deserialize,
    Serialize,
};

use self::transport::{
    OrderTransport,
    Transport,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct ProxyStateNotifyNode {
    pub external: SocketAddr,
    pub addr: SocketAddr,
    pub online: bool,
    pub index: u8,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Payload {
    ProxyStateNotify {
        nodes: Vec<ProxyStateNotifyNode>,
    },
    CreatePermission {
        id: u8,
        from: SocketAddr,
        peer: SocketAddr,
    },
}

impl TryFrom<&[u8]> for Payload {
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
        Ok(serde_json::from_slice(value)?)
    }
}

impl Into<Vec<u8>> for Payload {
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
    fn into(self) -> Vec<u8> {
        serde_json::to_vec(&self).expect("serde to json string failed!")
    }
}

pub trait RpcObserver: Send + Sync {
    fn on(&self, payload: Payload);
}

pub struct Rpc {
    sender: mpsc::UnboundedSender<(Payload, u8, bool)>,
}

impl Rpc {
    pub async fn new<T: RpcObserver + 'static>(
        addr: SocketAddr,
        observer: T,
    ) -> Result<Arc<Self>> {
        let (sender, mut receiver) =
            mpsc::unbounded_channel::<(Payload, u8, bool)>();

        let mut order_transport = OrderTransport::new(addr).await?;
        let mut transport = Transport::new(addr).await?;
        let observer = Arc::new(observer);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(ret) = order_transport.recv() => {
                        if let Some((buf, _)) = ret {
                            if let Ok(payload) = Payload::try_from(buf) {
                                observer.on(payload);
                            } else {
                                break;
                            }
                        }
                    }
                    Ok(ret) = transport.recv() => {
                        if let Some((buf, _)) = ret {
                            if let Ok(payload) = Payload::try_from(buf) {
                                observer.on(payload);
                            } else {
                                break;
                            }
                        }
                    }
                    Some((payload, to, is_order)) = receiver.recv() => {
                        let buf: Vec<u8> = payload.into();
                        if is_order {
                            if order_transport.send(&buf, to).await.is_err() {
                                break;
                            }
                        } else {
                            if transport.send(&buf, to).await.is_err() {
                                break;
                            }
                        }
                    }
                    else => {
                        break;
                    }
                };
            }
        });

        Ok(Arc::new(Self {
            sender,
        }))
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
    pub fn send_with_order(&self, payload: Payload, to: u8) -> Result<()> {
        self.sender.send((payload, to, true))?;
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
    pub fn send(&self, payload: Payload, to: u8) -> Result<()> {
        self.sender.send((payload, to, false))?;
        Ok(())
    }
}
