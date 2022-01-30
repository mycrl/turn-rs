mod controller;
mod accepter;
mod server;
mod state;
mod env;

use async_nats::connect;
use anyhow::Result;
use env::Environment;
use controller::*;
use state::State;

#[tokio::main]
#[rustfmt::skip]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_module_path(false)
        .init();
   
    let env = Environment::new();
    let conn = connect(env.nats.as_str()).await?;
    let controller = Publish::new(&env, conn.clone());
    let state = State::new(&env, &controller);
    
    server::run(env.clone(), state.clone()).await?;
    create_subscribe(&env, conn, state.clone()).await?;
    state.run().await?;
    Ok(())
}
