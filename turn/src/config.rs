use structopt::StructOpt;
use serde::Deserialize;
use anyhow::Result;
use std::{
    fs::File, 
    io::Read,
    sync::Arc,
    net::SocketAddr,
};

/// config model.
#[structopt(
    name = "Mysticeti",
    version = "0.1.0",
    author = "Mr.Panda <xivistudios@gmail.com>",
    about = "Rust ❤️ STUN/TURN Server"
)]
#[derive(StructOpt, Deserialize)]
pub struct Conf {
    #[structopt(short, long)]
    #[structopt(help = "conf file path")]
    config: Option<String>,
    
    /// specify the domain where the server is located. 
    /// for a single node, this configuration is fixed, 
    /// but each node can be configured as a different domain. 
    /// this is a good idea to divide the nodes by namespace.
    #[structopt(long)]
    #[structopt(default_value = "localhost")]
    #[structopt(help = "Service realm name")]
    #[serde(default = "default_realm")]
    pub realm: String,
    
    /// specify the node external address and port. 
    /// for the case of exposing the service to the outside, 
    /// you need to manually specify the server external IP 
    /// address and service listening port.
    #[structopt(long)]
    #[structopt(default_value = "127.0.0.1:3478")]
    #[structopt(help = "Service external address and port")]
    #[serde(default = "default_addr")]
    pub local: SocketAddr,
    
    /// the address and port bound by UDP Server. 
    /// currently, it does not support binding multiple 
    /// addresses at the same time. the bound address 
    /// supports ipv4 and ipv6.
    #[structopt(long)]
    #[structopt(default_value = "127.0.0.1:3478")]
    #[structopt(help = "Service bind address and port")]
    #[serde(default = "default_addr")]
    pub listen: SocketAddr,
    
    /// specify the remote control service. 
    /// the control service is very important. 
    /// if it is separated from it, 
    /// the service will only have the basic STUN binding function. 
    /// functions such as authorization authentication and port 
    /// allocation require communication with the control center.
    #[structopt(long)]
    #[structopt(default_value = "127.0.0.1:8080")]
    #[structopt(help = "HTTP external URL of the control service")]
    #[serde(default = "default_controls")]
    pub controls: SocketAddr,
    
    /// tshe buffer size is used to determine the maximum 
    /// data allocation size (byte) owned by each thread pool. 
    /// in actual use, it is recommended to configure this 
    /// value to 4096. a larger space will be easier to deal 
    /// with more complex MTU situations, although most of 
    /// the time The space utilization rate is not high.
    #[structopt(long)]
    #[structopt(default_value = "1280")]
    #[structopt(help = "UDP read buffer size")]
    #[serde(default = "default_buffer")]
    pub buffer: usize,
    
    /// by default, the thread pool is used to process UDP packets. 
    /// because UDP uses SysCall to ensure concurrency security, 
    /// using multiple threads may not bring a very significant 
    /// performance improvement, but setting the number of CPU 
    /// cores can process data to the greatest extent package.
    #[structopt(long)]
    #[structopt(help = "Runtime threads size")]
    pub threads: Option<usize>,
}

impl Conf {
    /// create config model.
    ///
    /// the configuration supports reading from cli or configuration file. 
    /// when specifying the --config/-f parameter, 
    /// other cli parameters will be ignored. 
    /// the configuration file will overwrite all parameter configurations. 
    /// at the same time, the configuration file path can be specified 
    /// by setting the `MYSTICETI_CONFIG` environment variable.
    pub fn new() -> Result<Arc<Self>> {
        let opt = Conf::from_args();
        Ok(Arc::new(match opt.config {
            Some(p) => read_file(p)?,
            None => match std::env::var("MYSTICETI_CONFIG") {
                Ok(p) => read_file(p)?,
                Err(_) => opt
            }
        }))
    }
}

/// read configure file.
///
/// read the configuration from the configuration file, 
/// there may be cases where the parse fail.
#[inline(always)]
fn read_file(path: String) -> Result<Conf> {
    log::info!("load conf file {:?}", &path);
    let mut buf = String::new();
    let mut file = File::open(path)?;
    file.read_to_string(&mut buf)?;
    Ok(toml::from_str(&buf)?)
}

/// realm needs to be clearly configured, the default 
/// value provided here only provides the default behavior.
fn default_realm() -> String {
    "localhost".to_string()
}

/// for security reasons, the network port is not open 
/// to the outside world by default, 
/// only the local port is bound.
fn default_addr() -> SocketAddr {
    "127.0.0.1:3478".parse().unwrap()
}

/// assume that the MTU is 1280 bytes, 
/// because IPv6 requires that the MTU of each 
/// connection in the network must be 1280 or greater.
fn default_buffer() -> usize {
    1280
}

fn default_controls() -> SocketAddr {
    "127.0.0.1:8080".parse().unwrap()
}