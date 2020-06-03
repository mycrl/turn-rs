mod router;
mod server;

use std::io::Error;
use configure::Configure;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let configure = Configure::generate();
    server::run(configure).await
}
