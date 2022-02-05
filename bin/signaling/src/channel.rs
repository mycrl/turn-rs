use tokio::sync::mpsc::*;
use anyhow::{
    anyhow,
    Result,
};

/// signals render channel.
pub type Rx = UnboundedReceiver<ChannelSignal>;

/// inner channel signals.
#[derive(Debug, PartialEq, Eq)]
pub enum ChannelSignal {
    Body(String),
    Close,
}

/// signals sender channel.
pub struct Tx(
    pub UnboundedSender<ChannelSignal>
);

impl Tx {
    /// send message to channel.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    ///
    /// let (sender, mut render) = unbounded_channel();
    /// let tx = Tx(sender);
    /// 
    /// tx.send(ChannelSignal::Close).unwrap();
    /// assert_eq!(render.blocking_recv(), Some(ChannelSignal::Close));
    /// ```
    #[rustfmt::skip]
    pub fn send(&self, signal: ChannelSignal) -> Result<()> {
        self.0.send(signal).map_err(|_| {
            anyhow!("channel send error!")
        })
    }
}

impl Clone for Tx {
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    ///
    /// let (sender, mut render) = unbounded_channel();
    /// let tx = Tx(sender);
    /// let tx2 = tx.clone();
    /// 
    /// tx.send(ChannelSignal::Close).unwrap();
    /// tx2.send(ChannelSignal::Body("a".to_string())).unwrap();
    /// 
    /// assert_eq!(render.blocking_recv(), Some(ChannelSignal::Close));
    /// assert_eq!(render.blocking_recv(), Some(ChannelSignal::Body("a".to_string())));
    /// ```
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
