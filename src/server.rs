// use.
use tokio::prelude::stream;
use tokio::net::TcpStream;
use tokio::net::TcpListener;
use tokio_codec::BytesCodec;
use tokio_codec::Decoder;
use futures::future::lazy;
use futures::Stream;
use futures::Future;
use futures::Sink;
use std::io::Error;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
// use std::sync::mpsc::Receiver;
use crate::CONFIGURE;
use crate::distributor::Distributor;
use crate::rtmp::Rtmp;
use crate::websocket::WebSocket;
use crate::configure::Listener;
use crate::distributor::Matedata;


/// # TCP Server Loop.
pub struct Servers {
    pub distributor: Distributor,
    pub listeners: Vec<Listener>
}


/// # Listener TCP Socket.
pub trait ListenerSocket {
    fn listener(self, sender: Sender<Matedata>);
}


impl ListenerSocket for Listener {

    /// tokio run worker.
    /// process socket.
    fn listener(self, sender: Sender<Matedata>) {
        let address_str = format!("{}{:?}", self.host, self.port);
        let address = &address_str.parse().unwrap();
        let incoming = TcpListener::bind(address).unwrap().incoming();
        tokio::spawn(incoming.map_err(drop)
        .for_each(move |socket| {
            match self.genre.as_str() {
                "push" => process_push(socket, sender),
                _ => process_server(socket)
            };

            Ok(())
        }));
    }
}


impl Servers {

    /// Merge all server options.
    pub fn merge_options (push: &Vec<Listener>, server: &Vec<Listener>) -> Vec<Listener> {
        let mut options = vec![];
        options.extend_from_slice(push);
        options.extend_from_slice(server);
        options
    }
    
    /// Create server connection loop.
    pub fn create () -> Self {
        let push = &CONFIGURE.push.clone();
        let server = &CONFIGURE.server;
        let listeners = Servers::merge_options(push, server);
        let distributor = Distributor::new();
        Servers { listeners, distributor }
    }

    /// Run work.
    pub fn work (self) {
        tokio::run(lazy(move || {
            for listen in self.listeners {
                listen.listener(Sender::clone(&self.distributor.channel.tx));
            }

            Ok(())
        }));
    }
}


/// Processing socket connection.
/// handling events and states that occur on the socket.
fn process_server (socket: TcpStream) {
    let address = socket.peer_addr().unwrap().to_string();
    let (writer, reader) = BytesCodec::new().framed(socket).split();
    let (socket_sender, socket_receiver) = mpsc::channel();
    let mut consumer = Rtmp::new(address.to_string(), socket_sender);
    
    // spawn socket data work.
    let socket_data_work = reader
    .for_each(move |bytes| Ok({ consumer.decoder(bytes); })) // decode bytes.
    .and_then(|()| { Ok(()) }) // socket received FIN packet and closed connection.
    .or_else(|err| { Err(err) }) // socket closed with error.
    .then(|_result| { Ok(()) }); // socket closed with result.

    // spawn socket write work.
    let socket_write_work = stream::iter_ok::<_, Error>(socket_receiver)
    .map(|bytes_mut| bytes_mut.freeze()) // BytesMut -> Bytes.
    .fold(writer, |writer, bytes| writer.send(bytes).and_then(|writer| writer.flush()) ) // Bytes -> send + flush.
    .and_then(|writer| Ok({ drop(writer); })) // channel receiver slose -> sink slose.
    .or_else(|_| Ok(())); // drop err.

    // spawn thread.
    tokio::spawn(socket_data_work);
    tokio::spawn(socket_write_work);
}


/// Processing socket connection.
/// handling events and states that occur on the socket.
fn process_push (socket: TcpStream, producer: Sender<Matedata>) {
    let address = socket.peer_addr().unwrap().to_string();
    let (writer, reader) = BytesCodec::new().framed(socket).split();
    let (socket_sender, socket_receiver) = mpsc::channel();
    let mut consumer = WebSocket::new(address.to_string(), socket_sender);
    
    // spawn socket data work.
    let socket_data_work = reader
    .for_each(move |bytes| Ok({ consumer.decoder(bytes); })) // decode bytes.
    .and_then(|()| { Ok(()) }) // socket received FIN packet and closed connection.
    .or_else(|err| { Err(err) }) // socket closed with error.
    .then(|_result| { Ok(()) }); // socket closed with result.

    // spawn socket write work.
    let socket_write_work = stream::iter_ok::<_, Error>(socket_receiver)
    .map(|bytes_mut| bytes_mut.freeze()) // BytesMut -> Bytes.
    .fold(writer, |writer, bytes| writer.send(bytes).and_then(|writer| writer.flush()) ) // Bytes -> send + flush.
    .and_then(|writer| Ok({ drop(writer); })) // channel receiver slose -> sink slose.
    .or_else(|_| Ok(())); // drop err.

    // spawn thread.
    tokio::spawn(socket_data_work);
    tokio::spawn(socket_write_work);
}