use std::fs::File;
use std::io::Read;


/// # Push Stream Config.
#[derive(Deserialize, Clone)]
pub struct Listener {
    pub protocol: String,
    pub genre: String,
    pub code: String,
    pub host: String,
    pub port: u32
}


/// # Live Pool
#[derive(Deserialize)]
pub struct Pool {
    pub bytes: u8
}


/// # Project Config.
#[derive(Deserialize)]
pub struct Config {
    pub push: Vec<Listener>,
    pub server: Vec<Listener>,
    pub pool: Pool
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