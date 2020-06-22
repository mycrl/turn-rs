use std::io::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    Ok(stun::start_server("0.0.0.0:3478").await?)
}
