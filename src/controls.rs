use super::config::Conf;
use serde::Deserialize;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;

/// 认证信息
///
/// * `password` 密钥
/// * `group` 分组ID
/// * `port` 分配端口
#[derive(Deserialize, Debug)]
pub struct Auth {
    pub password: String,
    pub group: u32,
}

/// 控制器
///
/// 外部控制API抽象
///
/// * `conf` 配置
/// * `req` 请求池
pub struct Controls {
    conf: Arc<Conf>,
    req: Client,
}

impl Controls {
    /// 创建实例
    ///
    /// # Example
    ///
    /// ```
    /// use super::*;
    ///
    /// let conf = config::new().unwrap();
    /// let controls = Controls::new(conf);
    /// ```
    pub fn new(conf: Arc<Conf>) -> Arc<Self> {
        Arc::new(Self {
            req: Client::new(),
            conf,
        })
    }

    /// 获取认证信息
    ///
    /// # Example
    ///
    /// ```
    /// use super::*;
    ///
    /// let conf = config::new().unwrap();
    /// let controls = Controls::new(conf);
    /// let addr = "127.0.0.1:8080".parse().unwrap();
    /// let auth = controls.auth("panda", addr);
    /// ```
    #[rustfmt::skip]
    pub async fn auth(&self, u: &str, a: &str) -> Result<Auth> {
       let res = self.req
           .get(&self.conf.controls)
           .query(&[
               ("type", "auth"),
               ("realm", &self.conf.realm),
               ("username", u),
               ("addr", a)
           ]).send().await?
           .json::<Auth>().await?;
        Ok(res)
    }
}
