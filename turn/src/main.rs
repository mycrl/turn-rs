mod controller;
mod processor;
mod server;
mod router;
mod args;

use controller::*;
use anyhow::Result;
use args::Args;
use router::Router;
use trpc::Rpc;

#[tokio::main]
#[rustfmt::skip]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_module_path(false)
        .init();
   
    let args = Args::new();
    let rpc = Rpc::new(args.get_nats_config()).await?;
    let r = Router::new(
        &args, 
        rpc.caller(Auth::new(&args))
    );
    
    rpc
        .servicer(Close::new(&r, &args)).await?
        .servicer(State::new(&r, &args)).await?
        .servicer(Node::new(&r, &args)).await?;
    server::run(&args, &r).await?;
    r.run().await?;
    Ok(())
}
