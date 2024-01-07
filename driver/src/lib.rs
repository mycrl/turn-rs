//! This is a simple distributed load balancing suite that contains both clients
//! and servers and supports unlimited cascading.
//!
//!
//! ## Toppology
//!
//! ![topoloy](./topology.webp)
//!
//! > Note that the communication protocol between balances uses udp and does
//! > not retransmit, if a packet is lost the current node will never enter the
//! > candidate again.
//!
//! #### Server
//!
//! You can deploy a Balance server in each region and in the server room where
//! the turn server is located and support unlimited cascading, but require each
//! Balance server to be externally accessible.
//!
//! #### Client
//!
//! The client provides SDKs and libraries that can be embedded into your own
//! applications. You can specify a top-level node on the client and initiate a
//! speed query. The client will first ask the top server, the top server will
//! reply to the client with all the subordinates of the current server, the
//! client will concurrently launch a query after getting the list of
//! subordinates, and the first server that replies will become the top server
//! again, and so on iteratively until the node where the turn server is located
//! is found.
//!
//!
//! #### Usage
//!
//! ```no_run
//! use std::net::SocketAddr;
//! use turn_driver::balance::Balance;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = "127.0.0.1:3001".parse::<SocketAddr>().unwrap();
//!     let balance = Balance::new(server).await.unwrap();
//!
//!     if let Ok(node) = balance.probe(10).await {
//!         // node is a socket addr.
//!     }
//! }
//! ```

pub mod balance;
pub mod controller;
pub mod hooks;

mod proto {
    tonic::include_proto!("turn");
    include!(concat!(env!("OUT_DIR"), "/balance.rs"));
}
