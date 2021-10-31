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
   
   let e = Environment::new();
   let b = Bridge::new(&e).await?;
   let s = State::new(&e, &b);
   server::run(e, s.clone()).await?;
   s.run().await?;
   Ok(())
}
