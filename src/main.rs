mod state;
mod remux;
mod server;
mod payload;
mod controls;
mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .format_module_path(false)
        .init();
    
    let c = config::new()?;
    let s = state::State::new();
    let t = controls::Controls::new(c.clone());
    server::run(c, s.clone(), t).await?;
    s.run().await?;
    Ok(())
}
