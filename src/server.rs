// use.
use bytes::BytesMut;
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
use crate::CONFIGURE;
use crate::rtmp::Rtmp;
use crate::websocket::WebSocket;
use crate::configure::Listener;


/// # TCP Server Loop.
pub struct Servers {
    pub distributor: Distributor,
    pub listeners: Vec<Listener>
}


/// # Listener TCP Socket.
pub trait ListenerSocket {
    fn listener(self, sender: Sender<DataType>);
}


impl ListenerSocket for Listener {

    /// tokio run worker.
    /// process socket.
    fn listener(self, sender: Sender<DataType>) {
        let address_str = format!("{}:{:?}", self.host, self.port);
        let address = &address_str.parse().unwrap();
        let incoming = TcpListener::bind(address).unwrap().incoming();
        tokio::spawn(incoming.map_err(drop)
        .for_each(move |socket| {
            match self.genre.as_str() {
                "push" => process_push(socket, Sender::clone(&sender)),
                _ => process_server(socket)
            };

            Ok(())
        }));
    }
}


impl Servers {
    
    /// Create server connection loop.
    pub fn create () -> Self {
        let listeners = CONFIGURE.server.clone();
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
fn process_push (socket: TcpStream, pool_sender: Sender<DataType>) {
    let address = socket.peer_addr().unwrap().to_string();
    let (writer, reader) = BytesCodec::new().framed(socket).split();
    let (socket_sender, socket_receiver) = mpsc::channel();
    let (mate_sender, mate_receiver) = mpsc::channel();
    let mut consumer = Rtmp::new(address.to_string(), mate_sender);
    
    // spawn socket data work.
    let socket_data_work = reader
    .for_each(move |bytes| Ok({ consumer.decoder(bytes); })) // decode bytes.
    .and_then(|()| { Ok(()) }) // socket received FIN packet and closed connection.
    .or_else(|err| { Err(err) }) // socket closed with error.
    .then(|_result| { Ok(()) }); // socket closed with result.

    // spawn socket write work.
    let socket_write_work = stream::iter_ok::<_, Error>(socket_receiver)
    .map(|bytes_mut: BytesMut| bytes_mut.freeze()) // BytesMut -> Bytes.
    .fold(writer, |writer, bytes| writer.send(bytes).and_then(|writer| writer.flush()) ) // Bytes -> send + flush.
    .and_then(|writer| Ok({ drop(writer); })) // channel receiver slose -> sink slose.
    .or_else(|_| Ok(())); // drop err.

    // spawn thread.
    tokio::spawn(socket_data_work);
    tokio::spawn(socket_write_work);
    tokio::spawn(lazy(move || {
        for receive in mate_receiver {
            match receive {
                DataType::BytesMut(bytes) => { socket_sender.send(bytes).unwrap(); },
                DataType::Matedata(_) => { pool_sender.send(receive).unwrap(); },
                DataType::Crated(_) => { pool_sender.send(receive).unwrap(); }
            };
        }

        Ok(())
    }));
}


/// Processing socket connection.
/// handling events and states that occur on the socket.
fn process_server (socket: TcpStream) {
    let address = socket.peer_addr().unwrap().to_string();
    let (writer, reader) = BytesCodec::new().framed(socket).split();
    let (sender, receiver) = mpsc::channel();
    let mut consumer = WebSocket::new(address.to_string(), sender);
    
    // spawn socket data work.
    let socket_data_work = reader
    .for_each(move |bytes| Ok({ consumer.decoder(bytes); })) // decode bytes.
    .and_then(|()| { Ok(()) }) // socket received FIN packet and closed connection.
    .or_else(|err| { Err(err) }) // socket closed with error.
    .then(|_result| { Ok(()) }); // socket closed with result.

    // spawn socket write work.
    let socket_write_work = stream::iter_ok::<_, Error>(receiver)
    .map(|bytes_mut: BytesMut| bytes_mut.freeze()) // BytesMut -> Bytes.
    .fold(writer, |writer, bytes| writer.send(bytes).and_then(|writer| writer.flush()) ) // Bytes -> send + flush.
    .and_then(|writer| Ok({ drop(writer); })) // channel receiver slose -> sink slose.
    .or_else(|_| Ok(())); // drop err.

    // spawn thread.
    tokio::spawn(socket_data_work);
    tokio::spawn(socket_write_work);
}