#[macro_use]
extern crate lazy_static;

mod handshake;
mod message;
mod session;

use bytes::BytesMut;
pub use handshake::Handshake;
pub use session::Session;
use transport::{Flag, Payload};

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
