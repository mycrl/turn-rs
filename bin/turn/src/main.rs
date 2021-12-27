mod accepter;
mod server;
mod bridge;
mod state;
mod env;

use anyhow::Result;
use env::Environment;
use bridge::Bridge;
use state::State;

#[tokio::main]
#[rustfmt::skip]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_module_path(false)
        .init();
   
    let env = Environment::new();
    let bridge = Bridge::new(&env).await?;
    let state = State::new(&env, &bridge);
    server::run(env, state.clone()).await?;
    state.run().await?;
    Ok(())
}
