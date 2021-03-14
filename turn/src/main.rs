mod state;
mod server;
mod config;
mod rpc;
mod hub;

#[tokio::main]
#[rustfmt::skip]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .format_module_path(false)
        .init();
    
    let c = config::Conf::new()?;
    let s = state::State::new();
    let t = rpc::Rpc::new(&c, &s).await?;
    server::run(c, s.clone(), t).await?;
    s.run().await?;
    Ok(())
}
