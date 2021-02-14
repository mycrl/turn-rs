use std::str::from_utf8 as str_from_utf8;
use num_enum::TryFromPrimitive;
use tokio::sync::RwLock;
use serde_json as Json;
use anyhow::{
    Result,
    Error,
    anyhow
};

use std::{
    collections::HashMap,
    convert::TryFrom,
    future::Future,
    sync::Arc
};

use serde::{
    de::DeserializeOwned,
    Serialize
};

use tokio::net::tcp::{
    OwnedReadHalf,
    OwnedWriteHalf
};

use tokio::sync::oneshot::{
    channel,
    Sender,
};

use tokio::sync::mpsc::{
    unbounded_channel,
    UnboundedSender,
};

use tokio::io::{
    AsyncReadExt,
    AsyncWriteExt
};

use bytes::{
    BytesMut,
    BufMut,
    Bytes,
    Buf
};

/// payload type flag.
#[repr(u8)]
#[derive(PartialEq, Eq)]
#[derive(TryFromPrimitive)]
pub enum Flag {
    /// rpc request.
    Request = 0,
    /// rpc ok response.
    Reply = 1,
    /// rpc error response.
    Error = 2
}

/// unique id.
#[derive(Default)]
struct Uid(u32);

/// RPC transport.
pub struct Rpc {
    /// callback channel table.
    call_table: RwLock<HashMap<u32, Sender<Result<Bytes, Error>>>>,
    /// event listener table.
    listener_table: RwLock<HashMap<u8, UnboundedSender<(u32, Bytes)>>>,
    /// tcp writer channel.
    writer: RwLock<OwnedWriteHalf>,
    /// tcp reader channel.
    reader: RwLock<OwnedReadHalf>,
    /// inner message id offset.
    uid: RwLock<Uid>,
}

impl Rpc {
    /// create rpc.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tokio::net::TcpStream;
    /// use transport::Rpc;
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut socket = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    ///     let (reader, writer) = socket.into_split();
    ///     Rpc::new(reader, writer);
    /// }
    /// ```
    pub fn new(reader: OwnedReadHalf, writer: OwnedWriteHalf) -> Arc<Self> {
        Arc::new(Self {
            listener_table: RwLock::new(HashMap::new()),
            call_table: RwLock::new(HashMap::new()),
            uid: RwLock::new(Uid::default()),
            reader: RwLock::new(reader),
            writer: RwLock::new(writer),
        })
    }

    /// run rpc.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tokio::net::TcpStream;
    /// use transport::Rpc;
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut socket = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    ///     let (reader, writer) = socket.into_split();
    ///     let transport = Rpc::new(reader, writer);
    ///     transport.run();
    /// }
    /// ```
    #[rustfmt::skip]
    pub fn run(self: Arc<Self>) -> Arc<Self> {
        let mut buf = BytesMut::with_capacity(1024);
        let s = self.clone();
        tokio::spawn(async move {
            loop { Self::result_hook(s.poll(&mut buf).await) }
        });

        self
    }

    /// bind event handler.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tokio::net::TcpStream;
    /// use transport::Rpc;
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut socket = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    ///     let (reader, writer) = socket.into_split();
    ///     let transport = Rpc::new(reader, writer).run();
    ///
    ///     transport.bind(0, |req: String| async move {
    ///         Ok("panda")
    ///     }).await;
    /// }
    /// ```
    #[rustfmt::skip]
    pub async fn bind<T, F, D, U>(self: Arc<Self>, kind: u8, mut handler: T)
    where
        D: Serialize + Send,
        U: DeserializeOwned + Send,
        F: Future<Output = Result<D, Error>> + Send,
        T: FnMut(U) -> F + Send + 'static
    {
        // create unbounded channel, 
        // insert the reader channel to the listener table temporarily.
        let (writer, mut reader) = unbounded_channel();
        self.listener_table.write().await.insert(kind, writer);
        
        // background tasks, 
        // repeated attempts to read messages in the pipeline, 
        // if there are predictable errors inside, 
        // it will not affect the continued execution.
    tokio::spawn(async move {loop {
        // read external request message.
        let (id, buf) = match reader.recv().await {
            None => continue,
            Some(m) => m
        };

        // try to serialize the message and give it to 
        // the event handler for execution.
        let result = match Json::from_slice(&buf[..]) {
            Ok(q) => (handler)(q).await,
            Err(_) => continue
        };

        let flag = match result {
            Ok(_) => Flag::Reply,
            Err(_) => Flag::Error,
        };

        // deserialize the event handler 
        // result into a string.
        let body = match result {
            Ok(r) => Json::to_string(&r).unwrap(),
            Err(e) => e.to_string(),
        };

        Self::result_hook(self.send(
            kind,
            flag,
            id,
            body.as_bytes()
        ).await)
    }});
        
    }

    /// call peer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tokio::net::TcpStream;
    /// use transport::Rpc;
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     let data = "username".to_string();
    ///     let mut socket = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    ///     let (reader, writer) = socket.into_split();
    ///     let transport = Rpc::new(reader, writer).run();
    ///     let name: String = transport.call(0, &data).await.unwrap();
    ///     println!("{:?}", name);
    /// }
    /// ```
    #[rustfmt::skip]
    pub async fn call<T, U>(&self, kind: u8, data: &T) -> Result<U>
    where
        T: Serialize,
        U: DeserializeOwned
    {
        // the offset of the sequence number increases, 
        // and returns to zero if it overflows.
        let mut uid = self.uid.write().await;
        uid.0 = if uid.0 >= u32::MAX { 0 } else { uid.0 + 1 };

        // create oneshot channel and 
        // write the pipeline to the internal call table.
        let (writer, reader) = channel();
        self.call_table.write().await.insert(uid.0, writer);

        // serialize the message and send it to the remote end via tcp.
        let req_buf = Json::to_vec(data)?;
        Self::result_hook(self.send(kind, Flag::Request, uid.0, &req_buf).await);

        // try to read the results in 
        // the channel and deserialize.
        let buf = reader.await??;
        let reply = Json::from_slice(&buf)?;
        Ok(reply)
    }

    /// send data to tcp socket.
    async fn send(&self, kind: u8, flag: Flag, id: u32, buf: &[u8]) -> Result<()> {
        let mut header = BytesMut::new();
        let mut socket = self.writer.write().await;

        // create message header.
        header.put_u32(buf.len() as u32);
        header.put_u8(kind);
        header.put_u8(flag as u8);
        header.put_u32(id);

        // submit in stages, and refresh to 
        // the opposite end after submission.
        socket.write_all(&header).await?;
        socket.write_all(&buf).await?;
        socket.flush().await?;

        Ok(())
    }

    /// inner poll and read data.
    #[rustfmt::skip]
    async fn poll(&self, buf: &mut BytesMut) -> Result<()> {
        self.reader.write().await.read_buf(buf).await?;
    loop {
        
        // check whether the buffer size meets the basic requirements, 
        // if not, then jump out of the loop.
        if buf.len() <= 10 {
            break;
        }

        // get message size.
        let size = u32::from_be_bytes([
            buf[0],
            buf[1],
            buf[2],
            buf[3]
        ]) as usize;

        // check the buffer size and confirm 
        // whether the message has arrived completely.
        if size + 10 > buf.len() {
            break;
        }

        // because the get size is for peek, 
        // so here is manually consumed u32 size.
        buf.advance(4);

        // get message action.
        // get message type, skip the unsupported type.
        let kind = buf.get_u8();
        let flag = match Flag::try_from(buf.get_u8()) {
            Err(_) => continue,
            Ok(f) => f
        };
        
        // get message uid.
        // get message data.
        let id = buf.get_u32();
        let body = buf.split_to(size).freeze();

        match flag {
            Flag::Request => self.process_request(kind, id, body).await,
            Flag::Reply => self.process_reply(id, body).await,
            Flag::Error => self.process_error(id, body).await
        };
    }

        Ok(())
    }
    
    #[rustfmt::skip]
    async fn process_request(&self, kind: u8, id: u32, body: Bytes) -> Option<()> {
        let mut listener_table = self.listener_table.write().await;
        listener_table.get_mut(&kind)?.send((id, body)).unwrap();
        None
    }

    #[rustfmt::skip]
    async fn process_reply(&self, id: u32, body: Bytes) -> Option<()> {
        let mut call = self.call_table.write().await;
        call.remove(&id)?.send(Ok(body)).unwrap();
        None
    }
    
    #[rustfmt::skip]
    async fn process_error(&self, id: u32, body: Bytes) -> Option<()> {
        let e = anyhow!(str_from_utf8(&body[..]).ok()?.to_string());
        let mut call = self.call_table.write().await;
        call.remove(&id)?.send(Err(e)).unwrap();
        None
    }
    
    fn result_hook(res: Result<()>) {
        if let Err(_) = res {
            std::process::abort();
        }
    }
}
