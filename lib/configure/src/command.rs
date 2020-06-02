use clap::{App, Arg}; 

/// 包含一些固定的项目信息
///
/// 版本，项目名，作者，简介.
const VERSION: &str = "0.1";
const APP_NAME: &str = "Quasipaa";
const AUTHOR: &str = "Mr.Panda <xivistudios@gmail.com>";
const ABOUT: &str = "Distributed real-time streaming media server cluster, support RTMP, http-flv, WebRTC...";

/// 命令行参数处理
pub struct Command;
impl Command {
    /// 获取命令行的配置文件参数
    ///
    /// 返回配置文件地址，
    /// 这是一个强制要求的参数，不能为空.
    pub fn configure() -> String {
        let arg = Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true);
        let matches = App::new(APP_NAME)
           .version(VERSION)
           .about(ABOUT)
           .author(AUTHOR)
           .arg(arg)
           .get_matches();
        matches.value_of("config")
            .expect("Configuration file is necessary")
            .to_string()
    }
}
