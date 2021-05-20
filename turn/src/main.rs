mod state;
mod server;
mod config;
mod proto;
mod broker;

#[tokio::main]
#[rustfmt::skip]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .format_module_path(false)
        .init();
    
    let c = config::Configure::new()?;
    let b = broker::Broker::new(&c).await?;
    let s = state::State::new(&c, &b);
    server::run(c, s.clone()).await?;
    s.run().await?;
    Ok(())
}
