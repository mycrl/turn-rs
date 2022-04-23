mod env;

use std::sync::Arc;
use tokio::net::TcpListener;
use env::Environment;
use anyhow::Result;
use signaling::*;
use trpc::*;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_module_path(false)
        .init();
    
    let env = Environment::new();
    let session = Router::new();
    let rpc = Rpc::new(&env.nats).await?;
    let listener = TcpListener::bind(env.listening).await?;
    log::info!(
        "signaling listen [{}]",
        env.listening
    );
    
    let caller = Arc::new(rpc.caller(Auth::new(&env.realm)));
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(Connection::launch(
            stream, 
            session.clone(), 
            caller.clone(),
            env.get_ws_config(),
        ));
    }

    Ok(())
}
