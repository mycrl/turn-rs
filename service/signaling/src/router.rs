use tokio::sync::RwLock;
use anyhow::Result;
use crate::channel::{
    ChannelSignal,
    Tx
};

use std::{
    collections::HashMap,
    sync::Arc,
};

/// message router.
pub struct Router {
    sessions: RwLock<HashMap<String, Tx>>,
}

impl Router {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { 
            sessions: RwLock::new(HashMap::with_capacity(4096)) 
        })
    }

    /// register session sender channel.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    ///
    /// let router = Router::new();
    /// let (sender, render) = unbounded_channel();
    /// let tx = Tx(sender);
    /// 
    /// router.register("panda", tx).await?;
    /// ```
    #[rustfmt::skip]
    pub async fn register(&self, uid: &str, sender: Tx) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(sender) = sessions.get(uid) {
            let _ = sender.send(ChannelSignal::Close);
        }

        sessions.insert(
            uid.to_string(), 
            sender
        );
        
        log::info!(
            "uid [{}] connected",
            uid
        );
        
        Ok(())
    }

    /// send message to target session.
    /// 
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    /// use tokio::runtime::Runtime;
    /// 
    /// Runtime::new().unwrap().block_on(async {
    ///     let router = Router::new();
    ///     let (sender, mut render) = unbounded_channel();
    /// 
    ///     router.register("panda", Tx(sender)).await.unwrap();
    ///     router.send_to("panda", "heelo".to_string()).await.unwrap();
    /// 
    ///     assert_eq!(render.recv().await, Some(ChannelSignal::Body("heelo".to_string())));
    /// });
    /// ```
    pub async fn send_to(&self, uid: &str, body: String) -> Result<()> {
        let mut close = None;
        if let Some(sender) = self.sessions.read().await.get(uid) {
            if sender.send(ChannelSignal::Body(body)).is_err() {
                close = Some(uid.to_string());
            }
        }

        if let Some(uid) = close {
            self.remove_vec(vec![uid]).await;
        }

        Ok(())
    }

    /// broadcast message to all session.
    /// 
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    /// use tokio::runtime::Runtime;
    ///
    /// Runtime::new().unwrap().block_on(async {
    ///     let router = Router::new();
    ///     let (sender, mut render) = unbounded_channel();
    /// 
    ///     router.register("panda", Tx(sender)).await.unwrap();
    ///     router.broadcast("test", "heelo".to_string()).await.unwrap();
    /// 
    ///     assert_eq!(render.recv().await, Some(ChannelSignal::Body("heelo".to_string())));
    /// });
    /// ```
    #[rustfmt::skip]
    pub async fn broadcast(&self, from: &str, body: String) -> Result<()> {
        let mut closes = Vec::with_capacity(10);
        for (uid, sender) in self.sessions.read().await.iter() {
            if uid != from && sender.send(ChannelSignal::Body(body.clone())).is_err() {
                closes.push(uid.clone());
            }
        }

        if !closes.is_empty() {
            self.remove_vec(closes).await;
        }

        Ok(())
    }

    /// remove sessions.
    /// 
    /// # Examples
    ///
    /// ```ignore
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    ///
    /// let router = Router::new();
    /// let (sender, render) = unbounded_channel();
    /// let tx = Tx(sender);
    /// 
    /// router.register("panda", tx).await.unwrap();
    /// // router.remove_vec(vec!["panda".to_string()]).await.unwrap();
    /// ```
    pub async fn remove_vec(&self, uids: Vec<String>) {
        let mut sessions = self.sessions.write().await;
        for uid in uids {
            sessions.remove(&uid);
        }
    }
}
