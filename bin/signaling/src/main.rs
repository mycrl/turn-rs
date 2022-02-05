mod env;

use tokio::net::TcpListener;
use env::Environment;
use anyhow::Result;
use signaling::{
    Controller,
    Connection,
    Router,
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_module_path(false)
        .init();
    
    let env = Environment::new();
    let session = Router::new();
    let controller = Controller::new(&env.nats, &env.realm).await?;
    let listener = TcpListener::bind(env.listening).await?;
    log::info!(
        "signaling listen [{}]",
        env.listening
    );
    
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(Connection::launch(
            stream, 
            session.clone(), 
            controller.clone(),
            env.get_ws_config(),
        ));
    }

    Ok(())
}
