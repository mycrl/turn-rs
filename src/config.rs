use structopt::StructOpt;
use serde::Deserialize;
use anyhow::Result;
use std::{
    fs::File, 
    io::Read,
    sync::Arc,
    net::SocketAddr,
};

// 配置
//
// * `realm` 服务域  
// * `listen` 服务器绑定地址  
// * `local` 本地对外IP地址  
// * `controls` 外部控制配置  
// * `udp` udp配置
#[structopt(
    name = "Mysticeti",
    version = "0.1.0",
    author = "Mr.Panda <xivistudios@gmail.com>",
    about = "Rust ❤️ WebRTC STUN/TURN Server"
)]
#[derive(StructOpt, Deserialize)]
pub struct Conf {
    #[structopt(short, long)]
    #[structopt(help = "conf file path")]
    config: Option<String>,
    // 指定服务器所在域，对于单个节点来说，这个配置
    // 是固定的，但是可以将每个节点配置为不同域，这
    // 是一个将节点按命名空间划分的好主意
    #[structopt(long)]
    #[structopt(default_value = "localhost")]
    #[structopt(help = "Service realm name")]
    #[serde(default = "default_realm")]
    pub realm: String,
    // 指定节点对外地址和端口，服务器并不会自动推测
    // 这个值，对于将服务对外暴露的情况来说，你需要
    // 手动指定为服务器对外IP地址和服务监听端口
    #[structopt(long)]
    #[structopt(default_value = "127.0.0.1:3478")]
    #[structopt(help = "Service external address and port")]
    #[serde(default = "default_addr")]
    pub local: SocketAddr,
    // UDP Server绑定的地址和端口，目前不支持同时
    // 绑定多个地址，绑定地址支持ipv4和ipv6
    #[structopt(long)]
    #[structopt(default_value = "127.0.0.1:3478")]
    #[structopt(help = "Service bind address and port")]
    #[serde(default = "default_addr")]
    pub listen: SocketAddr,
    // 指定远程控制服务，控制服务非常重要，脱离它服务
    // 将只具有基本STUN绑定功能，权限认证以及端口分配
    // 等功能都要求与控制中心通信
    #[structopt(long)]
    #[structopt(default_value = "127.0.0.1:8080")]
    #[structopt(help = "HTTP external URL of the control service")]
    #[serde(default = "default_controls")]
    pub controls: SocketAddr,
    // 缓冲区大小用于确定每个线程池拥有的最大数据分配
    // 大小(byte)，在实际使用中，推荐将这个值配置为4096，
    // 较大的空间将容易应对更复杂的MTU情况，虽然大部分时候
    // 部分空间的利用率不高
    #[structopt(long)]
    #[structopt(default_value = "1280")]
    #[structopt(help = "UDP read buffer size")]
    #[serde(default = "default_buffer")]
    pub buffer: usize,
    // 默认使用线程池处理UDP数据包，因为UDP存在
    // SysCall来确定并发安全性，所以使用多个线程有
    // 可能并不会带来显著的性能提升，不过设置为CPU
    // 核心数可以最大化并行数据包的处理和解析
    #[structopt(long)]
    #[structopt(help = "Runtime threads size")]
    pub threads: Option<usize>,
}

/// 创建配置
///
/// 配置支持从CLI或者配置文件中读取，
/// 当指定--config/-f参数的时候将忽略其他
/// 参数，配置文件覆盖所有参数配置，同时也
/// 可以用过 `MYSTICETI_CONFIG` 环境变量
/// 来设置配置文件路径
///
/// # Unit Test
///
/// ```test(new)
/// use super::*;
/// 
/// let conf = new().unwrap();
/// assert_eq!(conf.realm, "localhost".to_string());
/// assert_eq!(conf.local, "127.0.0.1:3478".parse().unwrap());
/// assert_eq!(conf.listen, "127.0.0.1:3478".parse().unwrap());
/// assert_eq!(conf.controls, "127.0.0.1:8080".parse().unwrap());
/// assert_eq!(conf.threads, None);
/// assert_eq!(conf.config, None);
/// assert_eq!(conf.buffer, 1280);
/// ```
pub fn new() -> Result<Arc<Conf>> {
    let opt = Conf::from_args();
    Ok(Arc::new(match opt.config {
        Some(p) => read_file(p)?,
        None => match std::env::var("MYSTICETI_CONFIG") {
            Ok(p) => read_file(p)?,
            Err(_) => opt
        }
    }))
}

/// 读取配置文件
///
/// 从配置文件中读取配置
/// 可能会存在解析失败的情况
#[inline(always)]
fn read_file(path: String) -> Result<Conf> {
    log::info!("load conf file {:?}", &path);
    let mut buf = String::new();
    let mut file = File::open(path)?;
    file.read_to_string(&mut buf)?;
    Ok(toml::from_str(&buf)?)
}

// 默认域
//
// 域需要明确配置，此处提供的
// 默认值只是提供了默认行为
fn default_realm() -> String {
    "localhost".to_string()
}

// 默认绑定地址
//
// 出于安全性考虑，网络端口默认
// 不对外开放，只绑定本地端口
fn default_addr() -> SocketAddr {
    "127.0.0.1:3478".parse().unwrap()
}

// 默认控制中心地址
//
// 这只是默认值，并不具备实际作用，
// 但是为默认配置提供了可能性
fn default_controls() -> SocketAddr {
    "127.0.0.1:8080".parse().unwrap()
}

// 默认缓冲区大小
//
// 默认大小假设MTU为1280字节，因为IPv6要求
// 网络中每个连接必须具有1280或者更大的MTU
fn default_buffer() -> usize {
    1280
}
