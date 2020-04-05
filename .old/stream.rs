// use.
use bytes::Bytes;
use rml_rtmp::sessions::StreamMetadata;
use rml_rtmp::time::RtmpTimestamp;
use crate::{ Rx, Tx };
use futures::prelude::*;
use futures::sync::mpsc;
use futures::stream::Stream;
use crate::handshake::Handshakes;
use crate::session::Session;
use std::sync::Arc;
use std::sync::Mutex;
use crate::shared::Shared;
use futures::try_ready;
use crate::bytes_stream::BytesStream;
use crate::shared::Client;
use std::rc::Rc;


pub enum Message {
    Raw(Bytes),
    Metadata(String, StreamMetadata),
    Audio(String, Bytes, RtmpTimestamp),
    Video(String, Bytes, RtmpTimestamp),
    Pull(String, String)
}


pub struct Socket {
    pub write: Tx,
    pub rx: Rx,
    pub reader: Rx,
    pub socket: BytesStream,
    pub handshake: Handshakes,
    pub session: Session,
    pub shared: Arc<Mutex<Shared>>
}


impl Socket {
    pub fn new (socket: BytesStream, shared: Arc<Mutex<Shared>>) -> Self {
        let (tx, rx) = mpsc::unbounded();
        let (write, reader) = mpsc::unbounded();
        let handshake = Handshakes::new(tx.clone());
        let session = Session::new(tx.clone());
        Socket { write, reader, rx, socket, handshake, session, shared }
    }
}


impl Future for Socket {
    type Item = ();
    type Error = ();

    fn poll (&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(Some(message)) = self.rx.poll().unwrap() {
            match message {
                Message::Raw(bytes) => {
                    self.socket.fill_write_buffer(bytes.to_vec().as_slice());
                },
                Message::Metadata(key, data) => {
                    let mut shared = self.shared.lock().unwrap();
                    if let Some(clients) = shared.clients.get_mut(&key) {
                        for client in clients {
                            client.tx.unbounded_send(Message::Metadata(key.clone(), data.clone())).unwrap();
                        }
                    }
                },
                Message::Audio(key, data, time) => {
                    let mut shared = self.shared.lock().unwrap();
                    if let Some(clients) = shared.clients.get_mut(&key) {
                        for client in clients {
                            client.tx.unbounded_send(Message::Audio(key.clone(), data.clone(), time.clone())).unwrap();
                        }
                    }
                },
                Message::Video(key, data, time) => {
                    let mut shared = self.shared.lock().unwrap();
                    if let Some(clients) = shared.clients.get_mut(&key) {
                        for client in clients {
                            client.tx.unbounded_send(Message::Video(key.clone(), data.clone(), time.clone())).unwrap();
                        }
                    }
                },
                Message::Pull(uid, key) => {
                    let mut shared = self.shared.lock().unwrap();
                    match shared.clients.get_mut(&key) {
                        Some(clients) => {
                            clients.push(Client { uid, tx: self.write.clone() });
                        },
                        None => {
                            shared.clients.insert(key, vec![
                                Client { uid, tx: self.write.clone() }
                            ]);
                        }
                    }
                }
            }
        }

        while let Async::Ready(Some(message)) = self.reader.poll().unwrap() {
            match message {
                Message::Metadata(_, data) => {
                    if let Some(id) = self.session.stream_id {
                        self.session.session.send_metadata(id, Rc::new(data)).unwrap();
                    }
                },
                Message::Audio(_, data, time) => {
                    if let Some(id) = self.session.stream_id {
                        self.session.session.send_audio_data(id, data, time, true).unwrap();
                    }
                },
                Message::Video(_, data, time) => {
                    if let Some(id) = self.session.stream_id {
                        self.session.session.send_video_data(id, data, time, true).unwrap();
                    }
                },
                _ => ()
            }
        }

        self.socket.poll_flush().unwrap();
        match try_ready!(self.socket.poll()) {
            Some(data) => {
                let mut data_copy = data.to_vec();
                if self.handshake.completed == false {
                    self.handshake.process(&mut data_copy);
                }

                if self.handshake.completed == true {
                    self.session.process(data_copy);
                }
            },
            None => ()
        };

        Ok(Async::Ready(()))
    }
}