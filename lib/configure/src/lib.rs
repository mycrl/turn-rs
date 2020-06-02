mod command;
pub mod model;

use std::fs;
use command::Command;
pub use model::ConfigureModel;

/// 配置处理
pub struct Configure;
impl Configure {
    /// 生成配置
    ///
    /// 从命令行参数获取配置文件地址，
    /// 解析之后返回配置文件模型和数据.
    ///
    /// TODO: 目前所有的配置都定义在配置文件内，
    /// 后期计划增加直接在命令行定义的方式.
    pub fn generate() -> ConfigureModel {
        let path = Command::configure();
        let file = fs::read_to_string(path).unwrap();
        toml::from_str(&file).unwrap()
    }
}
