use std::fs::File;
use std::io::Read;


/// # Push Stream Config.
/// 
/// * `host` push stream listen host.
/// * `port` push stream listen port.
#[derive(Deserialize)]
pub struct Push {
    pub host: String,
    pub port: u32
}


/// # Live Server Config.
/// 
/// * `host` live server listen host.
/// * `port` live server listen port.
#[derive(Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u32
}


/// # Project Config.
/// 
/// * `push` `{Push}` push stream config.
/// * `server` `{Server}` live server config.
#[derive(Deserialize)]
pub struct Config {
    pub push: Push,
    pub server: Server
}


impl Config {

    /// Read configure file. 
    /// 
    /// ## example
    /// ```
    /// let configure: Config = Config::from("./configure.toml");
    /// configure.host;
    /// ```
    pub fn from (path: &'static str) -> Config {
        let mut file = File::open(path).unwrap();
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).unwrap();
        let value: Config = toml::from_str(buffer.as_str()).unwrap();
        value
    }
}