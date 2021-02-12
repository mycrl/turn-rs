mod config;
mod controls;
mod payload;
mod hub;
mod server;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder().format_module_path(false).init();

    let c = config::new()?;
    let s = state::State::new();
    let t = controls::Controls::new(c.clone(), s.clone()).await?;
    server::run(c, s.clone(), t).await?;
    s.run().await?;
    Ok(())
}
