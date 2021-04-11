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
    
    let c = config::Conf::new()?;
    let t = broker::Broker::new(&c.controls).await?;
    let s = state::State::new(t);
    server::run(c, s.clone()).await?;
    s.run().await?;
    Ok(())
}
