mod accepter;
mod broker;
mod server;
mod state;
mod argv;

use anyhow::Result;
use broker::Broker;
use state::State;
use argv::Argv;

#[tokio::main]
#[rustfmt::skip]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_module_path(false)
        .init();
    
    let c = Argv::new();
    let b = Broker::new(&c).await?;
    let s = State::new(&c, &b);
    server::run(c, s.clone()).await?;
    s.run().await?;
    Ok(())
}
