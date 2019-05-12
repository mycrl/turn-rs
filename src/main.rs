// crate.
#[macro_use] 
extern crate serde_derive;
extern crate lazy_static;
extern crate tokio;
extern crate toml;
extern crate futures;
extern crate bytes;
extern crate tokio_codec;
extern crate rml_rtmp;
extern crate uuid;
extern crate mse_fmp4;
extern crate parking_lot;
extern crate httparse;
extern crate sha1;
extern crate base64;


// mod.
mod configure;
mod server;
mod rtmp;
mod ws;
mod pool;


// use.
use lazy_static::lazy_static;
use configure::Config;
use server::Servers;


// global static constant.
lazy_static!{
    // project config.
    pub static ref CONFIGURE: Config = Config::from("./configure.toml");
}


// main.
fn main () {
    Servers::create().work();
}