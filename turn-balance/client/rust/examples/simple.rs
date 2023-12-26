use turn_balance_client::Balance;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Ok(addr) = Balance::new("127.0.0.1:3001".parse()?)
        .await?
        .probe(10)
        .await
    {
        println!("found a node: addr={}", addr);
    } else {
        println!("not found");
    }

    Ok(())
}
