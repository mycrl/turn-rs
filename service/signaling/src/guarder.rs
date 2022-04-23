use std::sync::Arc;
use anyhow::{
    Result,
    anyhow,
};

use crate::{
    controller::AuthCaller,
    router::Router,
    channel::Tx
};

use http::{
    status::StatusCode,
    response,
};

use tokio::{
    task::block_in_place,
    runtime::Handle,
    sync::Mutex,
};

use tungstenite::handshake::server::{
    Callback,
    ErrorResponse,
    Request,
    Response,
};

/// websocket upgrade guarder.
pub struct Guarder {
    handle: Handle,
    router: Arc<Router>,
    uid: Arc<Mutex<String>>,
    caller: Arc<AuthCaller>,
    tx: Tx,
}

impl Guarder {
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
    /// while let Ok((stream, _)) = listener.accept().await {
    ///     tokio::spawn(async move {
    ///         let guarder = Guarder::new(&session.clone(), Handle::current(), Tx(sender));
    ///         Socket::new(accept_hdr_async(stream, guarder).await?);
    ///         Ok(())
    ///     });
    /// }
    /// ```
    #[rustfmt::skip]
    pub fn new(
        caller: Arc<AuthCaller>,
        router: Arc<Router>, 
        handle: Handle, 
        tx: Tx, 
        uid: Arc<Mutex<String>>,
    ) -> Self {
        Self { 
            caller,
            router, 
            handle,
            uid,
            tx,
        }
    }

    /// handle http request,
    /// authentication verification.
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
    /// while let Ok((stream, _)) = listener.accept().await {
    ///     tokio::spawn(async move {
    ///         let guarder = Guarder::new(&session.clone(), Handle::current(), Tx(sender));
    ///         Socket::new(accept_hdr_async(stream, guarder).await?);
    ///         Ok(())
    ///     });
    /// }
    /// ```
    pub fn handle(&self, req: &Request) -> Result<()> {
        block_in_place(move || self.handle.block_on(async move {
            let tx = self.tx.clone();
            let uid = req
                .uri()
                .path()
                .split('/')
                .nth(1)
                .ok_or_else(|| anyhow!("invalid url!"))?;
            if uid.is_empty() {
                return Err(anyhow!("invalid url!"))
            }

            let token = req
                .uri()
                .query()
                .ok_or_else(|| anyhow!("invalid token!"))?;
            if token.is_empty() {
                return Err(anyhow!("invalid token!"))
            }

            self.caller
                .call((
                    uid.to_string(), 
                    token.to_string()
                ))
                .await?;
            self.router
                .register(uid, tx)
                .await?;
            self.uid
                .lock()
                .await
                .push_str(uid);
            Ok(())
        }))
    }
    
    /// failed response builder.
    ///
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    /// use http::status::StatusCode;
    ///
    /// let res = Guarder::failed_builder("error!".to_string());
    /// assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    /// assert_eq!(res.body(), &Some("error!".to_string()));
    /// ```
    pub fn failed_builder(e: String) -> ErrorResponse {
        response::Builder::new()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Some(e))
            .unwrap()
    }
}

impl Callback for Guarder {
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
    /// while let Ok((stream, _)) = listener.accept().await {
    ///     tokio::spawn(async move {
    ///         let guarder = Guarder::new(&session.clone(), Handle::current(), Tx(sender));
    ///         Socket::new(accept_hdr_async(stream, guarder).await?);
    ///         Ok(())
    ///     });
    /// }
    /// ```
    #[rustfmt::skip]
    fn on_request(self, req: &Request, res: Response) -> Result<Response, ErrorResponse> {
        self.handle(req).map(|_| res).map_err(|e| {
            Self::failed_builder(e.to_string())
        })
    }
}
