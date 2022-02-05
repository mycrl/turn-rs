//! ## The signaling server
//!
//! Establishing a WebRTC connection between two devices requires the use 
//! of a signaling server to resolve how to connect them over the internet.
//! A signaling server's job is to serve as an intermediary to let two peers 
//! find and establish a connection while minimizing exposure of potentially 
//! private information as much as possible. How do we create this server and 
//! how does the signaling process actually work?
//!
//! First we need the signaling server itself. WebRTC doesn't specify a 
//! transport mechanism for the signaling information. You can use anything 
//! you like, from WebSocket to XMLHttpRequest to carrier pigeons to exchange 
//! the signaling information between the two peers.
//!
//! It's important to note that the server doesn't need to understand or 
//! interpret the signaling data content. Although it's SDP, even this doesn't 
//! matter so much: the content of the message going through the signaling server 
//! is, in effect, a black box. What does matter is when the ICE subsystem instructs 
//! you to send signaling data to the other peer, you do so, and the other peer 
//! knows how to receive this information and deliver it to its own ICE subsystem. 
//! All you have to do is channel the information back and forth. 
//! The contents don't matter at all to the signaling server.
//!
//!
//! ## The signaling protocol
//!
//! * `to`: target user id.  
//! * `from`: self user id.  
//! all other fields are preserved.

mod controller;
mod message;
mod channel;
mod guarder;
mod router;
mod socket;

pub use controller::*;
pub use message::*;
pub use channel::*;
pub use guarder::*;
pub use router::*;
pub use socket::*;
