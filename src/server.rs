// use.
use tokio::net::TcpStream;
use tokio::net::TcpListener;
use tokio_codec::BytesCodec;
use tokio_codec::Decoder;
use futures::future::lazy;
use futures::Stream;
use futures::Future;
use futures::Sink;
use bytes::BytesMut;
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
        let (writer, reader) = BytesCodec::new().framed(socket).split();
        let (sender, receiver) = mpsc::channel();
        let mut codec = RTMP::new();
        
        // spawn socket data work.
        tokio::spawn(reader.for_each(move |bytes| {
            // decode bytes.
            codec.decoder(bytes, Sender::clone(&sender));
            Ok(())
        }).and_then(|()| {
            // socket received FIN packet and closed connection.
            Ok(())
        }).or_else(|err| {
            // socket closed with error.
            Err(err)
        }).then(|_result| {
            // socket closed with result.
            Ok(())
        }));

        // spawn socket write work.
        tokio::spawn(tokio::prelude::stream::iter_ok::<_, Error>(receiver)
        .map(|bytes_mut| {
            // BytesMut -> Bytes.
            bytes_mut.freeze()
         }).fold(writer, |writer, bytes| {
            // Bytes -> send + flush.
            writer.send(bytes).and_then(|writer| writer.flush())
        }).and_then(|writer| {
            // channel receiver slose -> sink slose.
            drop(writer);
            Ok(())
        }).or_else(|err| {
            // Err -> ()
            println!("{:?}", err);
            Ok(())
        }));
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