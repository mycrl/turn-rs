mod router;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(server::run("127.0.0.1:1936".parse().unwrap()).await?)
}
