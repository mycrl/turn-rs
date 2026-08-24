use tonic::{Status, transport::Server};
use turn_server_sdk::{
    Credential, TurnHooksServer,
    protos::{Identifier, PasswordAlgorithm},
};

struct MyHooksServer;

#[tonic::async_trait]
impl TurnHooksServer for MyHooksServer {
    async fn register(
        &self,
        id: Identifier,
        realm: String,
        username: String,
        algorithm: PasswordAlgorithm,
    ) -> Result<Credential, Status> {
        println!(
            "Registering id={id:?}, realm={realm}, username={username}, algorithm={algorithm:?}"
        );

        // Implement your authentication logic here
        // For example, look up the user in a database
        Ok(Credential {
            password: "test".to_string(),
            realm,
        })
    }

    async fn on_allocated(&self, realm: String, id: Identifier, username: String, port: u16) {
        println!("Session allocated: realm={realm}, id={id:?}, username={username}, port={port}");
        // Handle allocation event (e.g., log to database, update metrics)
    }

    async fn on_channel_bind(&self, realm: String, id: Identifier, username: String, channel: u16) {
        println!("Channel bound: realm={realm}, id={id:?}, username={username}, channel={channel}");
    }

    async fn on_create_permission(
        &self,
        realm: String,
        id: Identifier,
        username: String,
        ports: Vec<u16>,
    ) {
        println!(
            "Permission created: realm={realm}, id={id:?}, username={username}, ports={ports:?}",
        );
    }

    async fn on_refresh(&self, realm: String, id: Identifier, username: String, lifetime: u32) {
        println!(
            "Session refreshed: realm={realm}, id={id:?}, username={username}, lifetime={lifetime}"
        );
    }

    async fn on_destroy(&self, realm: String, id: Identifier, username: String) {
        println!("Session destroyed: realm={realm}, id={id:?}, username={username}");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start the hooks server
    let mut server = Server::builder();
    let hooks = MyHooksServer;

    hooks
        .start_with_server(&mut server, "127.0.0.1:3000".parse()?)
        .await?;

    Ok(())
}
