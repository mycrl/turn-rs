use serde_derive::Deserialize;
use std::net::SocketAddr;

/// 交换中心配置
#[derive(Debug, Deserialize)]
pub struct Exchange {
    /// 对外绑定的地址
    #[serde(default = "Exchange::default_addr")]
    pub addr: String,
    
    ///对外绑定的端口
    #[serde(default = "Exchange::default_port")]
    pub port: u16
}

impl Exchange {
    /// 将内部的地址和端口
    /// 转为SocketAddr类型
    /// 
    /// 注意: 如果转换不成功将直接panic.
    pub fn to_addr(&self) -> SocketAddr {
        let mut addr = self.addr.clone();
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
        1936u16
    }
}

impl Default for Exchange {
    fn default() -> Self {
        Self {
            addr: Self::default_addr(),
            port: Self::default_port() 
        }
    }
}

/// 推流服务配置
#[derive(Debug, Deserialize)]
pub struct Publish {
    /// 对外绑定的地址
    #[serde(default = "Publish::default_addr")]
    pub addr: String,
    
    ///对外绑定的端口
    #[serde(default = "Publish::default_port")]
    pub port: u16
}

impl Publish {
    /// 将内部的地址和端口
    /// 转为SocketAddr类型
    /// 
    /// 注意: 如果转换不成功将直接panic.
    pub fn to_addr(&self) -> SocketAddr {
        let mut addr = self.addr.clone();
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
        1935u16
    }
}

impl Default for Publish {
    fn default() -> Self {
        Self {
            addr: Self::default_addr(),
            port: Self::default_port() 
        }
    }
}

/// 拉流服务配置
#[derive(Debug, Deserialize)]
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

/// 配置模型
#[derive(Debug, Deserialize)]
pub struct ConfigureModel {
    /// 交换中心服务
    #[serde(default)]
    pub exchange: Exchange,
    
    /// 推流服务
    #[serde(default)]
    pub publish: Publish,
    
    /// 拉流服务
    #[serde(default)]
    pub pull: Pull
}
