mod router;
mod server;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    server::run("127.0.0.1:1936".parse().unwrap()).await
}
