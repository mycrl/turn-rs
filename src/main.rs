// crate.
#[macro_use] 
extern crate serde_derive;
extern crate lazy_static;
extern crate tokio;
extern crate toml;
extern crate futures;
extern crate bytes;
extern crate tokio_codec;
extern crate rand;


// mod.
mod configure;
mod server;
mod rtmp;
mod util;
mod pool;


// use.
use lazy_static::lazy_static;
use configure::Config;
use server::Servers;
use std::io::Error;


// global static constant.
lazy_static!{
    // project config.
    pub static ref CONFIGURE: Config = Config::from("./configure.toml");
}


// main.
fn main () -> Result<(), Error> {
    Servers::create().work();
    Ok(())
}