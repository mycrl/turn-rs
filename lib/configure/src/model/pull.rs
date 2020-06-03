use serde_derive::Deserialize;
use std::net::SocketAddr;

/// 拉流服务配置
#[derive(Debug, Deserialize, Clone)]
pub struct Pull {
    /// 对外绑定的地址
    #[serde(default = "Pull::default_addr")]
    pub addr: String,
    
    ///对外绑定的端口
    #[serde(default = "Pull::default_port")]
    pub port: u16
}

impl Pull {
    /// 将内部的地址和端口
    /// 转为SocketAddr类型
    /// 
    /// 注意: 如果转换不成功将直接panic.
    pub fn to_addr(&self) -> SocketAddr {
        let mut addr = self.addr.clone();
        addr.push(':');
        addr.push_str(&self.port.to_string());
        addr.parse().unwrap()
    }
    
    /// 默认绑定地址
    ///
    /// 默认不公开绑定，
    /// 只允许本地访问.
    fn default_addr() -> String {
        "127.0.0.1".to_string()
    }
    
    /// 默认绑定端口
    fn default_port() -> u16 {
        80u16
    }
}

impl Default for Pull {
    fn default() -> Self {
        Self {
            addr: Self::default_addr(),
            port: Self::default_port() 
        }
    }
}