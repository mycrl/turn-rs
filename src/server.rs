// use.
use tokio::net::TcpStream;
use tokio::net::TcpListener;
use tokio_codec::BytesCodec;
use tokio_codec::Decoder;
use futures::future::lazy;
use futures::Stream;
use futures::Future;
use futures::Sink;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::io::Error;
use crate::CONFIGURE;
use crate::rtmp::RTMP;


/// # TCP Listener Context.
/// 
/// * `name` `{str}` tcp listener type.
/// * `address` `{String}` tcp listener address.
pub struct Listener {
    pub name: &'static str,
    pub address: String
}


/// # TCP Server Loop.
/// 
/// * `server` `{Listener}` tcp server.
/// * `push` `{Listener}` tcp push server.
pub struct Servers {
    pub server: Listener,
    pub push: Listener
}


/// # Listener TCP Socket.
/// 
/// * `listener` run worker.
pub trait ListenerSocket {
    fn listener(self);
}


impl ListenerSocket for Listener {

    /// tokio run worker.
    /// process socket.
    fn listener(self) {
        let address = &self.address.parse().unwrap();
        let incoming = TcpListener::bind(address).unwrap().incoming();
        tokio::spawn(incoming
        .map_err(|err| println!("error = {:?}", err))
        .for_each(move |socket| {
            Servers::process(socket, self.name);
            Ok(())
        }));
    }
}


impl Servers {
    
    /// Create server connection loop.
    /// 
    /// ## example
    /// ```
    /// Servers::create();
    /// ```
    pub fn create () -> Self {
        let server_addr = format!("{}:{:?}", &CONFIGURE.server.host, &CONFIGURE.server.port);
        let push_addr = format!("{}:{:?}", &CONFIGURE.push.host, &CONFIGURE.push.port);
        let server = Listener { address: server_addr, name: "server" };
        let push = Listener { address: push_addr, name: "push" };
        Servers { server, push }
    }

    /// Processing socket connection.
    /// handling events and states that occur on the socket.
    /// 
    /// ## example
    /// ```
    /// Servers::process(socket, "");
    /// ```
    pub fn process (socket: TcpStream, _name: &'static str) {
        let address = socket.peer_addr().unwrap().to_string();
        let (writer, reader) = BytesCodec::new().framed(socket).split();
        let (sender, receiver) = mpsc::channel();
        let mut codec = RTMP::new(address.to_string());
        
        // spawn socket data work.
        let socket_data_work = reader
        .for_each(move |bytes| { Ok({ codec.decoder(bytes, Sender::clone(&sender)); }) }) // decode bytes.
        .and_then(|()| { Ok(()) }) // socket received FIN packet and closed connection.
        .or_else(|err| { Err(err) }) // socket closed with error.
        .then(|_result| { Ok(()) }); // socket closed with result.

        // spawn socket write work.
        let socket_write_work = tokio::prelude::stream::iter_ok::<_, Error>(receiver)
        .map(|bytes_mut| bytes_mut.freeze()) // BytesMut -> Bytes.
        .fold(writer, |writer, bytes| {
            println!("发送数据");
            writer.send(bytes).and_then(|writer| writer.flush())
        }) // Bytes -> send + flush.
        .and_then(|writer| Ok({ drop(writer); })) // channel receiver slose -> sink slose.
        .or_else(|_| Ok(())); // drop err.

        // spawn thread.
        tokio::spawn(socket_data_work);
        tokio::spawn(socket_write_work);
    }

    /// Run work.
    /// 
    /// ## example
    /// ```
    /// Servers::create().work();
    /// ```
    pub fn work (self) {
        tokio::run(lazy(move || {
            self.server.listener();
            self.push.listener();
            Ok(())
        }));
    }
}