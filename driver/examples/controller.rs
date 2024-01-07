use turn_driver::controller::Controller;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ctr = Controller::new("http://localhost:3000").await?;
    let stats = ctr.get_stats().await?;
    println!("stats: {:?}", stats);

    Ok(())
}
