use num_enum::TryFromPrimitive;
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
    mem::transmute,
    sync::Arc
};

use serde::{
    de::DeserializeOwned,
    Serialize
};

use tokio::{
    net::TcpStream,
    sync::RwLock
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

/// 负载类型
///
/// * `Request` 请求
/// * `Reply` 正确响应
/// * `Err` 错误响应
#[repr(u8)]
#[derive(PartialEq, Eq)]
#[derive(TryFromPrimitive)]
enum Kind {
    Request = 0,
    Reply = 1,
    Err = 2
}

/// 请求ID
#[derive(Default)]
struct Uid {
    inner: u32
}

/// 缓冲区
#[derive(Default)]
struct Buffer {
    inner: BytesMut
}

/// RPC传输
///
/// * `call_stack` 回调栈表
/// * `listener` 监听器表
/// * `inner` TCP连接
/// * `buffer` 缓冲区
/// * `uid` 内部ID偏移量
pub struct Transport {
    call_stack: RwLock<HashMap<u32, Sender<Result<Bytes, Error>>>>,
    listener: RwLock<HashMap<u8, UnboundedSender<(u32, Bytes)>>>,
    inner: RwLock<TcpStream>,
    buffer: RwLock<Buffer>,
    uid: RwLock<Uid>,
}

impl Transport {
    pub fn new(socket: TcpStream) -> Arc<Self> {
        Arc::new(Self {
            call_stack: RwLock::new(HashMap::new()),
            buffer: RwLock::new(Buffer::default()),
            listener: RwLock::new(HashMap::new()),
            uid: RwLock::new(Uid::default()),
            inner: RwLock::new(socket),
        }).run()
    }

    #[rustfmt::skip]
    pub async fn bind<T, F, D, U>(self: Arc<Self>, kind: u8, mut handle: T)
    where
        D: Serialize + Send,
        U:DeserializeOwned + Send,
        F: Future<Output = Result<D, Error>> + Send,
        T: FnMut(U) -> F + Send + 'static
    {
        let (writer, mut reader) = unbounded_channel();
        self.listener.write().await.insert(kind, writer);

        tokio::spawn(async move { 
            loop {
                let (id, buf) = match reader.recv().await {
                    None => continue,
                    Some(m) => m
                };
                
                let req_buf = unsafe { transmute(&buf) };
                let result = match Json::from_slice(req_buf) {
                    Ok(q) => (handle)(q).await,
                    Err(_) => continue
                };

                if let Err(e) = self.listen_hook(kind, id, result).await {
                    log::error!("transport err: {:?}", e);
                }
            }
        });   
    }

    #[rustfmt::skip]
    pub async fn call<T, U>(&self, k: u8, data: &T) -> Result<U>
    where
        T: Serialize,
        U: DeserializeOwned
    {
        let mut uid = self.uid.write().await;
        uid.inner = if uid.inner >= u32::MAX { 0 } else { uid.inner + 1 };

        let (writer, reader) = channel();
        self.call_stack.write().await.insert(uid.inner, writer);

        let req_buf = Json::to_vec(data)?;
        self.send(k, Kind::Request, uid.inner, &req_buf).await?;

        let buf =reader.await??;
        let reply = Json::from_slice(&buf)?;
        Ok(reply)
    }

    async fn send(&self, k: u8, kind: Kind, id: u32, buf: &[u8]) -> Result<()> {
        let mut header = BytesMut::new();
        let mut socket = self.inner.write().await;

        header.put_u32(buf.len() as u32);
        header.put_u8(k);
        header.put_u8(kind as u8);
        header.put_u32(id);

        socket.write_all(&header).await?;
        socket.write_all(&buf).await?;
        socket.flush().await?;

        Ok(())
    }

    #[rustfmt::skip]
    async fn poll(&self) -> Result<()> {
        let mut buf = self.buffer.write().await;
        self.inner.write().await.read_buf(&mut buf.inner).await?;

        loop {
            if buf.inner.len() <= 10 {
                break;
            }

            let size = u32::from_be_bytes([
                buf.inner[0],
                buf.inner[1],
                buf.inner[2],
                buf.inner[3]
            ]) as usize;

            if size + 10 < buf.inner.len() {
                break;
            }

            buf.inner.advance(4);

            let name = buf.inner.get_u8();
            let kind = Kind::try_from(buf.inner.get_u8())?;
            let id = buf.inner.get_u32();
            let body = buf.inner.split_to(size).freeze();

            if kind == Kind::Request {
                if let Some(listen) = self.listener.write().await.get_mut(&name) {
                    listen.send((id, body)).unwrap();
                    continue;
                }
            }
            
            let call = match self.call_stack.write().await.remove(&id) {
                None => continue,
                Some(c) => c,
            };
            
            if kind == Kind::Reply {
                call.send(Ok(body)).unwrap();
                continue;
            }
            
            if kind == Kind::Err {
                let err = std::str::from_utf8(&body[..])?.to_string();
                call.send(Err(anyhow!(err))).unwrap();
            }
        }

        Ok(())
    }

    #[rustfmt::skip]
    async fn listen_hook<T>(&self, k: u8, id: u32, result: Result<T>) -> Result<()>
    where T : Serialize
    {
        let kind = match result {
            Ok(_) => Kind::Reply,
            Err(_) => Kind::Err,
        };

        let body = match result {
            Ok(r) => Json::to_string(&r)?,
            Err(e) => e.to_string(),
        };

        self.send(
            k, 
            kind, 
            id,
            body.as_bytes()
        ).await
    }

    #[rustfmt::skip]
    fn run(self: Arc<Self>) -> Arc<Self> {
        let handle = self.clone();
        tokio::spawn(async move {
            loop { let _ = handle.poll().await; }
        });
        
        self
    }
}
