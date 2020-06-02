mod handshake;
mod message;
mod session;

use super::{Codec, Packet};
use bytes::BytesMut;
pub use session::Session;
pub use handshake::Handshake;
use transport::{Flag, Payload, Transport};

/// process result
pub enum State {
    /// There are unprocessed data blocks.
    Overflow(BytesMut),
    /// There is a data block that needs to be returned to the peer.
    Callback(BytesMut),
    /// Clear buffer.
    /// Used to transfer handshake to session.
    Empty,
    /// Event message.
    Event(Payload, Flag),
}

/// Rtmp protocol processing
///
/// Input and output TCP data, the whole process is completed automatically.
/// At the same time, some key RTMP messages are returned.
pub struct Rtmp {
    handshake: Handshake,
    session: Session,
}

impl Rtmp {
    /// Handle Rtmp handshake
    ///
    /// Incoming writeable buffers and results will be done automatically.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rtmp::Rtmp;
    /// use bytes::BytesMut;
    ///
    /// let mut rtmp = Rtmp::default();
    /// let mut results = Vec::new();
    /// let mut buffer = BytesMut::from(b"");
    /// rtmp.process_handshake(&buffer, &results);
    /// ```
    pub fn process_handshake(&mut self, buffer: &mut BytesMut, receiver: &mut Vec<Packet>) {
        if let Some(states) = self.handshake.process(&buffer) {
            for state in states {
                if let Some(packet) = self.process_state(state, buffer) {
                    receiver.push(packet);
                }
            }
        }
    }

    /// Processing Rtmp messages
    ///
    /// Incoming writeable buffers and results will be done automatically.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rtmp::Rtmp;
    /// use bytes::BytesMut;
    ///
    /// let mut rtmp = Rtmp::default();
    /// let mut results = Vec::new();
    /// let mut buffer = BytesMut::from(b"");
    /// rtmp.process_session(&buffer, &results);
    /// ```
    pub fn process_session(&mut self, buffer: &mut BytesMut, receiver: &mut Vec<Packet>) {
        if let Some(states) = self.session.process(&buffer) {
            for state in states {
                if let Some(packet) = self.process_state(state, buffer) {
                    receiver.push(packet);
                }
            }
        }
    }

    /// The operation result returned by the processing module
    ///
    /// The results include multimedia data, overflow data,
    /// callback data, and clear control information.
    fn process_state(&mut self, state: State, buffer: &mut BytesMut) -> Option<Packet> {
        match state {
            // callback data
            // Data to be sent to the peer TcpSocket.
            State::Callback(callback) => Some(Packet::Peer(callback)),

            // Event message.
            State::Event(payload, flag) => {
                let data = Transport::packet(payload);
                Some(Packet::Core(data, flag))
            }

            // overflow data
            // Rewrite the buffer and pass the overflow data to the
            // next process to continue processing.
            State::Overflow(overflow) => {
                *buffer = BytesMut::from(&overflow[..]);
                None
            }

            // Special needs
            // Clear the buffer, no remaining data
            // needs to be processed.
            State::Empty => {
                buffer.clear();
                None
            }
        }
    }
}

impl Default for Rtmp {
    fn default() -> Self {
        Self {
            handshake: Handshake::new(),
            session: Session::new(),
        }
    }
}

impl Codec for Rtmp {
    fn parse(&mut self, buffer: &mut BytesMut) -> Vec<Packet> {
        let mut receiver = Vec::new();

        // The handshake is not yet complete,
        // Hand over to the handshake module to process Tcp data.
        if !self.handshake.completed {
            self.process_handshake(buffer, &mut receiver);
        }

        // The handshake is completed,
        // Process Rtmp messages.
        if self.handshake.completed {
            self.process_session(buffer, &mut receiver);
        }

        receiver
    }
}
