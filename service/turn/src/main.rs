mod controller;
mod accepter;
mod server;
mod router;
mod env;

use controller::*;
use anyhow::Result;
use env::Environment;
use router::Router;
use trpc::Rpc;

#[tokio::main]
#[rustfmt::skip]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_module_path(false)
        .init();
   
    let env = Environment::new();
    let rpc = Rpc::new(env.nats.as_str()).await?;
    let r = Router::new(
        &env, 
        rpc.caller(Auth::new(&env))
    );
    
    rpc
        .servicer(Close::new(&r, &env)).await?
        .servicer(State::new(&r, &env)).await?
        .servicer(Node::new(&r, &env)).await?;
    server::run(&env, &r).await?;
    r.run().await?;
    Ok(())
}
