#[macro_use]
extern crate lazy_static;

mod handshake;
mod message;
mod session;

pub use handshake::Handshake;
pub use session::Session;
use bytes::BytesMut;

/// Message flag
pub const FLAG_VIDEO: u8 = 0;
pub const FLAG_AUDIO: u8 = 1;
pub const FLAG_FRAME: u8 = 2;
pub const FLAG_PUBLISH: u8 = 3;
pub const FLAG_UNPUBLISH: u8 = 4;

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
    Event(BytesMut, u8),
}
