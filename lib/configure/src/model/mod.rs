mod exchange;
mod publish;
mod pull;

use serde_derive::Deserialize;
use exchange::Exchange;
use publish::Publish;
use pull::Pull;

/// 配置模型
#[derive(Debug, Deserialize, Clone)]
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
