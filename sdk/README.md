# Turn Server SDK

A Rust client SDK for interacting with the `turn-server` gRPC API exposed by the `turn-rs` project. This crate provides both client and server utilities for TURN server integration.

## Features

-   **TurnService Client**: Query server information, session details, and manage TURN sessions
-   **TurnHooksServer**: Implement custom authentication and event handling for TURN server hooks
-   **Password Generation**: Generate STUN/TURN authentication passwords using MD5 or SHA256

## Installation

Add turn-server-sdk to your `Cargo.toml`:

```sh
cargo add turn-server-sdk
```

## Client Usage

The `TurnService` client allows you to interact with a running TURN server's gRPC API:

```rust
use tonic::transport::Channel;
use turn_server_sdk::{
    TurnService,
    protos::{Identifier, Transport},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the TURN server gRPC endpoint
    let channel = Channel::from_static("http://127.0.0.1:3000")
        .connect()
        .await?;

    // Create a client
    let mut client = TurnService::new(channel);

    // Get server information
    let info = client.get_info().await?;
    println!("Server software: {}", info.software);

    let id = Identifier {
        source: "127.0.0.1".to_string(),
        external: "127.0.0.1".to_string(),
        interface: "127.0.0.1".to_string(),
        transport: Transport::Udp as i32,
    };

    // Query a session by ID
    let session = client.get_session(id.clone()).await?;
    println!("Session username: {}", session.username);

    // Get session statistics
    let stats = client.get_session_statistics(id.clone()).await?;
    println!("Bytes sent: {}", stats.send_bytes);

    // Destroy a session
    client.destroy_session(id).await?;

    Ok(())
}
```

## Server Usage (Hooks Implementation)

Implement the `TurnHooksServer` trait to provide custom authentication and handle TURN events. See also [`examples/simple.rs`](examples/simple.rs).

```rust
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
        realm: &str,
        username: &str,
        algorithm: PasswordAlgorithm,
    ) -> Result<Credential, Status> {
        // Implement your authentication logic here
        // For example, look up the user in a database
        Ok(Credential {
            password: "user-password".to_string(),
            realm: realm.to_string(),
        })
    }

    async fn on_allocated(&self, id: Identifier, username: String, port: u16) {
        println!("Session allocated: id={:?}, username={}, port={}", id, username, port);
        // Handle allocation event (e.g., log to database, update metrics)
    }

    async fn on_channel_bind(&self, id: Identifier, username: String, channel: u16) {
        println!("Channel bound: id={:?}, username={}, channel={}", id, username, channel);
    }

    async fn on_create_permission(&self, id: Identifier, username: String, ports: Vec<u16>) {
        println!("Permission created: id={:?}, username={}, ports={:?}", id, username, ports);
    }

    async fn on_refresh(&self, id: Identifier, username: String, lifetime: u32) {
        println!("Session refreshed: id={:?}, username={}, lifetime={}", id, username, lifetime);
    }

    async fn on_destroy(&self, id: Identifier, username: String) {
        println!("Session destroyed: id={:?}, username={}", id, username);
        // Handle session destruction (e.g., cleanup resources)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start the hooks server
    let mut server = Server::builder();
    let hooks = MyHooksServer;

    hooks.start_with_server(
        &mut server,
        "127.0.0.1:3000".parse()?,
    ).await?;

    Ok(())
}
```

## Password Generation

Generate STUN/TURN authentication passwords for long-term credentials:

```rust
use turn_server_sdk::{generate_password, protos::PasswordAlgorithm};

// Generate MD5 password (RFC 5389)
let md5_password = generate_password(
    "username",
    "password",
    "realm",
    PasswordAlgorithm::Md5,
);

// Generate SHA256 password (RFC 8489)
let sha256_password = generate_password(
    "username",
    "password",
    "realm",
    PasswordAlgorithm::Sha256,
);

// Access the password bytes
match md5_password {
    turn_server_sdk::Password::Md5(bytes) => {
        println!("MD5 password: {:?}", bytes);
    }
    turn_server_sdk::Password::Sha256(bytes) => {
        println!("SHA256 password: {:?}", bytes);
    }
}
```

## Event Handling

The `TurnHooksServer` trait provides hooks for various TURN server events:

-   `register`: Called when a client authenticates; return the credential used for message integrity
-   `on_allocated`: Called when a client allocates a relay port
-   `on_channel_bind`: Called when a channel is bound to a peer
-   `on_create_permission`: Called when permissions are created for peers
-   `on_refresh`: Called when a session is refreshed
-   `on_destroy`: Called when a session is destroyed

All event handlers are optional and have default no-op implementations. You only need to implement the ones you care about.

## Error Handling

Most operations return `Result<T, Status>` where `Status` is a gRPC status code. Common error scenarios:

-   `Status::not_found`: Session or resource not found
-   `Status::unavailable`: Server is not available
-   `Status::unauthenticated`: Authentication failed
-   `Status::internal`: Internal server error

## Protobuf bindings

This crate exposes generated protobuf types under `turn_server_sdk::protos`. gRPC transport types come from the `tonic` crate:

```rust
use tonic::transport::Channel;
use turn_server_sdk::protos;
```

## Documentation

For more detailed API documentation, see:

-   [API Documentation](https://docs.rs/turn-server-sdk) (when published)
-   [TURN Server Documentation](../README.md)
-   [RFC 8489](https://tools.ietf.org/html/rfc8489) - Session Traversal Utilities for NAT (STUN)
-   [RFC 8656](https://tools.ietf.org/html/rfc8656) - Traversal Using Relays around NAT (TURN)

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
