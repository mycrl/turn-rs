use turn_server_sdk::{
    Credential, TurnHooksServer,
    protos::PasswordAlgorithm,
    tonic::{Status, transport::Server},
};

struct MyHooksServer;

#[turn_server_sdk::tonic::async_trait]
impl TurnHooksServer for MyHooksServer {
    async fn get_password(
        &self,
        realm: &str,
        username: &str,
        algorithm: PasswordAlgorithm,
    ) -> Result<Credential, Status> {
        println!(
            "Getting password for realm={}, username={}, algorithm={:?}",
            realm, username, algorithm
        );

        // Implement your authentication logic here
        // For example, look up the user in a database
        Ok(Credential {
            password: "test".to_string(),
            realm: realm.to_string(),
        })
    }

    async fn on_allocated(&self, id: String, username: String, port: u16) {
        println!(
            "Session allocated: id={}, username={}, port={}",
            id, username, port
        );
        // Handle allocation event (e.g., log to database, update metrics)
    }

    async fn on_channel_bind(&self, id: String, username: String, channel: u16) {
        println!(
            "Channel bound: id={}, username={}, channel={}",
            id, username, channel
        );
    }

    async fn on_create_permission(&self, id: String, username: String, ports: Vec<u16>) {
        println!(
            "Permission created: id={}, username={}, ports={:?}",
            id, username, ports
        );
    }

    async fn on_refresh(&self, id: String, username: String, lifetime: u32) {
        println!(
            "Session refreshed: id={}, username={}, lifetime={}",
            id, username, lifetime
        );
    }

    async fn on_destroy(&self, id: String, username: String) {
        println!("Session destroyed: id={}, username={}", id, username);
        // Handle session destruction (e.g., cleanup resources)
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
