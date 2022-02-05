use tungstenite::protocol::WebSocketConfig;
use clap::Parser;
use std::{
    net::SocketAddr,
    sync::Arc,
};

#[derive(Parser, Debug)]
#[clap(
    name = "Signaling",
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS")
)]
pub struct Environment {
    /// listening:
    ///
    /// the address and port bound by http Server. currently, it does not support binding multiple addresses 
    /// at the same time. the bound address supports ipv4 and ipv6.
    #[clap(default_value = "127.0.0.1:80", env = "SIGAALING_LISTENING")]
    pub listening: SocketAddr,
    /// realm:
    ///
    /// specify the domain where the server is located. for a single node, this configuration is fixed, 
    /// but each node can be configured as a different domain. this is a good idea to divide the nodes by namespace.
    #[clap(default_value = "localhost", env = "SIGAALING_REALM")]
    pub realm: String, 
    /// nats:
    ///
    /// specify the remote control service. the control service is very important. if it is separated from it, 
    /// the service will only have the basic STUN binding function. functions such as authorization authentication 
    /// and port allocation require communication with the control center.
    #[clap(default_value = "127.0.0.1:4222", env = "SIGAALING_NATS")]
    pub nats: String,
    /// max send queue
    /// 
    /// The size of the send queue. You can use it to turn on/off the backpressure features. 
    /// None means here that the size of the queue is unlimited. The default value is the unlimited queue.
    #[clap(env = "SIGAALING_MAX_SEND_QUEUE", long)]
    pub max_send_queue: Option<usize>,
    /// max message size
    /// 
    /// The maximum size of a message. None means no size limit. The default value is 64 MiB which should
    /// be reasonably big for all normal use-cases but small enough to prevent memory eating by a malicious user.
    #[clap(env = "SIGAALING_MAX_MESSAGE_SIZE", long)]
    pub max_message_size: Option<usize>,
    /// max frame size
    /// 
    /// The maximum size of a single message frame. None means no size limit. The limit is for frame payload 
    /// NOT including the frame header. The default value is 16 MiB which should be reasonably big for all 
    /// normal use-cases but small enough to prevent memory eating by a malicious user.
    #[clap(env = "SIGAALING_MAX_FRAME_SIZE", long)]
    pub max_frame_size: Option<usize>,
    /// accept unmasked frames
    /// 
    /// When set to true, the server will accept and handle unmasked frames from the client. According to the RFC 6455, 
    /// the server must close the connection to the client in such cases, however it seems like there are some popular 
    /// libraries that are sending unmasked frames, ignoring the RFC. By default this option is set to false, 
    /// i.e. according to RFC 6455.
    #[clap(env = "SIGAALING_UNMASKED_FRAMES", long)]
    pub accept_unmasked_frames: bool,
}

impl Environment {
    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    ///
    /// let env = Environment::new();
    /// assert_eq!(env.listening, "127.0.0.1:80".parse().unwrap());
    /// ```
    pub fn new() -> Arc<Self> {
        Arc::new(Self::parse())
    }

    /// # Unit Test
    ///
    /// ```
    /// use signaling::*;
    ///
    /// let env = Environment::new();
    /// let config = env.get_ws_config();
    /// 
    /// assert_eq!(config.max_send_queue, None);
    /// assert_eq!(config.max_message_size, None);
    /// assert_eq!(config.max_frame_size, None);
    /// assert_eq!(config.accept_unmasked_frames, false);
    /// ```
    pub fn get_ws_config(&self) -> WebSocketConfig {
        WebSocketConfig {
            max_send_queue: self.max_send_queue,
            max_message_size: self.max_message_size,
            max_frame_size: self.max_frame_size,
            accept_unmasked_frames: self.accept_unmasked_frames,
        }
    }
}
