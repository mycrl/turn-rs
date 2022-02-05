use std::sync::Arc;
use anyhow::{
    anyhow,
    Result
};

use futures_util::{
    stream::StreamExt,
    sink::SinkExt,
};

use crate::{
    controller::Controller,
    message::Payload,
    guarder::Guarder,
    router::Router,
};

use crate::channel::{
    ChannelSignal,
    Rx,
    Tx,
};

use tokio::{
    net::TcpStream,
    runtime::Handle,
    sync::mpsc::*,
    sync::Mutex,
};

use tokio_tungstenite::{
    accept_hdr_async_with_config,
    WebSocketStream,
};

use tungstenite::protocol::{
    WebSocketConfig,
    Message,
};

/// websocket single socket.
pub struct Socket {
    inner: WebSocketStream<TcpStream>
}

impl Socket {
    pub fn new(inner: WebSocketStream<TcpStream>) -> Self {
        Self { inner }
    }

    /// send message to socket.
    /// 
    /// # Examples
    /// 
    /// ``` ignore
    /// let mut socket = Socket::new(websocket);
    /// 
    /// socket.send(Message::Text("hello".to_string())).await?;
    /// ```
    pub async fn send(&mut self, message: Message) -> Result<()> {
        self.inner.send(message).await?;
        Ok(())
    }

    /// read message in socket.
    /// 
    /// # Examples
    /// 
    /// ``` ignore
    /// let mut socket = Socket::new(websocket);
    /// 
    /// while let Some(message) = socket.read().await {
    ///     // message is tungstenite enum type.
    /// }
    /// ```
    pub async fn read(&mut self) -> Option<Message> {
        if let Some(Ok(message)) = self.inner.next().await {
            return Some(message)
        }
    
        None
    }

    /// close socket.
    /// 
    /// # Examples
    /// 
   /// ``` ignore
    /// let mut socket = Socket::new(websocket);
    /// 
    /// socket.send(Message::Text("hello".to_string())).await?;
    /// socket.close();
    /// 
    /// // panic
    /// // socket.send(Message::Text("hello".to_string())).await?;
    /// ```
    pub async fn close(&mut self) -> Result<()> {
        self.inner.close(None).await?;
        Ok(())
    }
}

/// http single connection.
pub struct Connection {
    router: Arc<Router>,
    socket: Socket,
    uid: String,
    rx: Rx,
} 

impl Connection {
    /// create websocket connection from tcp socket.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    /// use tokio::net::*;
    ///
    /// let (sender, render) = unbounded_channel();
    /// let session = Router::new();
    /// let listener = TcpListener::bind(env.listen).await?;
    /// let controller = Controller::new(&env.nats, &env.realm).await?;
    /// 
    /// while let Ok((stream, _)) = listener.accept().await {
    ///     tokio::spawn(async move {
    ///         Connection::new(stream, router.clone(), controller.clone()).await?;
    ///         Ok(())
    ///     });
    /// }
    /// ```
    #[rustfmt::skip]
    pub async fn new(
        stream: TcpStream, 
        router: Arc<Router>, 
        controller: Arc<Controller>,
        config: WebSocketConfig,
    ) -> Result<Self> {
        let uid = Arc::new(Mutex::new(String::with_capacity(100)));
        let (tx, rx) = unbounded_channel();
        let guarder = Guarder::new(
            controller,
            router.clone(), 
            Handle::current(), 
            Tx(tx),
            uid.clone(),
        );
        
        let socket = Socket::new(
            accept_hdr_async_with_config(
                stream, 
                guarder, 
                Some(config)
            ).await?
        );
        
        let uid = Arc::try_unwrap(uid)
            .map_err(|_| anyhow!("channel send error!"))?
            .into_inner();
        Ok(Self {
            router,
            socket,
            uid,
            rx
        })
    }

    /// handle channel inner signal message.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    /// use tokio::net::*;
    ///
    /// let (sender, render) = unbounded_channel();
    /// let session = Router::new();
    /// let listener = TcpListener::bind(env.listen).await?;
    /// let controller = Controller::new(&env.nats, &env.realm).await?;
    /// 
    /// while let Ok((stream, _)) = listener.accept().await {
    ///     tokio::spawn(async move {
    ///         let mut conn = Connection::new(stream, router.clone(), controller.clone()).await?;
    ///         conn.handle_signal(ChannelSignal::Close).await?;
    ///         Ok(())
    ///     });
    /// }
    /// ```
    pub async fn handle_signal(&mut self, signal: ChannelSignal) -> Result<()> {
        match signal {
            ChannelSignal::Body(body) => self.socket.send(Message::Text(body)).await?,
            ChannelSignal::Close => self.socket.close().await?,
        }

        Ok(())
    }
    
    /// handle websocket message.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    /// use tokio::net::*;
    ///
    /// let (sender, render) = unbounded_channel();
    /// let session = Router::new();
    /// let listener = TcpListener::bind(env.listen).await?;
    /// let controller = Controller::new(&env.nats, &env.realm).await?;
    /// 
    /// while let Ok((stream, _)) = listener.accept().await {
    ///     tokio::spawn(async move {
    ///         let mut conn = Connection::new(stream, router.clone(), controller.clone()).await?;
    ///         conn.handle_msg("hello".to_string()).await?;
    ///         Ok(())
    ///     });
    /// }
    /// ```
    pub async fn handle_msg(&mut self, text: String) -> Result<()> {
        match Payload::get_to(text.as_str()) {
            Ok(Some(to)) => self.router.send_to(&to, text).await?,
            Ok(None) => self.router.broadcast(&self.uid, text).await?,
            _ => ()
        }

        Ok(())
    }

    /// read message in socket.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    /// use tokio::net::*;
    ///
    /// let (sender, render) = unbounded_channel();
    /// let session = Router::new();
    /// let listener = TcpListener::bind(env.listen).await?;
    /// let controller = Controller::new(&env.nats, &env.realm).await?;
    /// 
    /// while let Ok((stream, _)) = listener.accept().await {
    ///     tokio::spawn(async move {
    ///         Connection::new(stream, router.clone(), controller.clone())
    ///             .await?
    ///             .poll()
    ///             .await?;
    ///         Ok(())
    ///     });
    /// }
    /// ```
    pub async fn poll(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                Some(signal) = self.rx.recv() => self.handle_signal(signal).await?,
                Some(Message::Text(text)) = self.socket.read() => self.handle_msg(text).await?,
            }
        }
    }

    /// read message in socket.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use signaling::*;
    /// use tokio::sync::mpsc::*;
    /// use tokio::net::*;
    ///
    /// let (sender, render) = unbounded_channel();
    /// let session = Router::new();
    /// let listener = TcpListener::bind(env.listen).await?;
    /// let controller = Controller::new(&env.nats, &env.realm).await?;
    /// 
    /// while let Ok((stream, _)) = listener.accept().await {
    ///     tokio::spawn(Connection::launch(stream, router.clone(), controller.clone()));
    /// }
    /// ```
    pub async fn launch(
        stream: TcpStream, 
        router: Arc<Router>, 
        controller: Arc<Controller>,
        config: WebSocketConfig,
    ) -> Result<()> {
        Connection::new(
            stream, 
            router, 
            controller, 
            config
        )
        .await?
        .poll()
        .await
    }
}
