# turn-rs AI Coding Guide

A high-performance TURN server implementation in Rust focused on WebRTC use cases, achieving 40M channel data forwards/sec and <35μs forwarding latency.

## Coding Style Preferences

-   **Comments**: Use English for all code comments and documentation
-   **Testing**: Include detailed assertions with descriptive messages explaining what is being verified
-   **Validation**: Check specific attribute values in responses, not just presence/absence
-   **Output**: Provide informative test output showing actual values (addresses, ports, lifetimes, etc.)

## Architecture Overview

### Workspace Structure (3 crates + main binary)

```
turn-server/           # Main binary - config, server transport, RPC/hooks
├── crates/codec/      # STUN/TURN protocol encoding/decoding (RFC 5389, 5766, 6062, 6156)
├── crates/service/    # Core business logic - session management, routing, port allocation
└── sdk/               # gRPC client SDK for external control
```

**Key principle**: Zero-copy message parsing with lifetime-bound `Message<'a>` and `ChannelData<'a>` types. Buffers are reused via `BytesMut` in `Router`.

### Critical Data Flow

1. **Transport → Router → Service**: TCP/UDP servers accept connections, each connection gets a `Router<T>` instance
2. **Router decoding**: `Decoder` determines `Message` vs `ChannelData` by first byte flags (0=STUN, else channel)
3. **Session lookup**: `Identifier { source: SocketAddr, interface: SocketAddr }` is the session key
4. **Routing decision**: Returns `Response<'a>` with `Target { endpoint, relay }` to indicate where to forward

### Session Management Pattern

Sessions are stored in `SessionManager<T>` with **virtual port allocation** (no real system ports). States:

-   `Unauthenticated { nonce }`: Initial state, generates STUN nonce
-   `Authenticated { allocate_port, channels, permissions, ... }`: After successful ALLOCATE request

Port allocation uses a bitmap (`PortAllocator`) for efficient O(1) allocation from configurable `PortRange` (default 49152-65535).

## Development Workflows

### Building & Testing

```bash
# Build with all features (UDP, TCP, SSL, RPC)
cargo build --release --all-features

# Build specific features
cargo build --features "udp,tcp,rpc"

# Run codec tests (uses binary samples from tests/samples/)
cargo test -p turn-server-codec

# Run service tests
cargo test -p turn-server-service

# Benchmarks (codec performance)
cargo bench -p turn-server-codec
```

### Running the Server

```bash
# Requires config file (see turn-server.toml for schema)
cargo run --release -- --config=turn-server.toml

# Docker
docker pull ghcr.io/mycrl/turn-server
docker run -v ./turn-server.toml:/etc/turn-server/config.toml ghcr.io/mycrl/turn-server

# Linux service install
./install-service.sh  # Compiles and installs systemd service
```

### Configuration Deep Dive

-   **Transport**: `[[server.interfaces]]` array supports mixed UDP/TCP/TLS, multiple network interfaces
-   **Authentication priority**: static-credentials → static-auth-secret (TURN REST API) → hooks (gRPC)
-   **Features**: Controlled by Cargo features (`udp`, `tcp`, `ssl`, `rpc`) which enable/disable modules at compile time

## Project-Specific Conventions

### Message Handling Pattern (in `routing.rs`)

Every STUN method handler follows this structure:

```rust
async fn method_name<'a, T>(req: Request<'_, 'a, T, Message<'_>>) -> Option<Response<'a>>
where T: ServiceHandler
{
    // 1. Extract required attributes, reject if missing
    let attr = req.payload.get::<AttributeType>()?;

    // 2. Verify authentication (long-term credentials via HMAC)
    let Some((username, password)) = req.verify().await else {
        return reject(req, ErrorType::Unauthorized);
    };

    // 3. Business logic via SessionManager
    req.state.manager.operation(req.id, ...);

    // 4. Call handler hooks (for external events/logging)
    req.state.handler.on_event(req.id, username, ...);

    // 5. Build response using MessageEncoder with borrowed buffer
    {
        let mut message = MessageEncoder::extend(METHOD_RESPONSE, req.payload, req.encode_buffer);
        message.append::<Attribute>(value);
        message.flush(Some(&password)).ok()?;
    }

    Some(Response::Message { method, bytes: req.encode_buffer, target })
}
```

### ServiceHandler Trait (Extension Point)

Implement this trait in `src/handler.rs` to customize server behavior:

-   `get_password()`: Authentication source (static, REST API, or gRPC hooks)
-   `on_allocated()`, `on_channel_bind()`, etc.: Event notifications sent via gRPC to external services when `rpc` feature enabled

### Codec Zero-Copy Design

`Decoder` maintains reusable `Attributes` buffer to avoid allocations. Parsed messages **borrow** from input:

```rust
// Attributes are ranges into original buffer
pub struct Attributes(Vec<(AttributeType, Range<usize>)>);

// Messages hold references, not owned data
pub struct Message<'a> {
    buffer: &'a [u8],
    attributes: &'a Attributes,
    // ...
}
```

**When editing codec**: Preserve lifetime bounds and ensure `decode()` doesn't clone bytes.

### Edition 2024 & Modern Rust

Uses `edition = "2024"` in all `Cargo.toml` files. Leverage:

-   `if let` chains in let_chains feature contexts (see `indication()` function in `routing.rs`)
-   `mimalloc` global allocator for performance
-   `parking_lot` for faster RwLocks over std

## External Integrations

### gRPC API (requires `rpc` feature)

-   **Control plane**: `TurnService` exposes GetInfo, GetSession, DestroySession RPCs
-   **Hooks plane**: `TurnHooksService` receives events (OnAllocatedEvent, etc.) from server
-   **Proto schema**: `protos/server.proto` compiled via `tonic-prost-build` in `build.rs`

When adding RPC methods:

1. Update `server.proto`
2. Rebuild (auto-generates code via build script)
3. Implement in `src/rpc.rs` for control or `RpcHooksService` for hooks

### Statistics & Prometheus

Statistics tracking (`src/statistics.rs`) uses atomics for lock-free counters. Keyed by `Identifier` for per-session metrics (bytes/packets tx/rx).

## Testing & Binary Samples

-   **tests/samples/**: Pre-captured binary STUN messages used in `crates/codec/tests/stun.rs`
-   **Integration tests**: `crates/service/tests/turn.rs` creates in-memory `Service` with mock handler
-   **WEBRTC_DEMO.html**: Browser test page for live TURN server validation

When adding protocol features, capture real wire format to `tests/samples/` for regression tests.

## Performance Considerations

-   **Single-threaded processing**: Each Router owns its `Decoder` and `BytesMut` - not shared across threads
-   **Pre-allocated buffers**: `BytesMut::with_capacity(4096)` in Router, `Vec::with_capacity(20)` for Attributes
-   **Virtual ports only**: No kernel port allocation per session, all data forwarded through server's listening ports
-   **Raspberry Pi tested**: Designed for resource-constrained environments

## Common Gotchas

1. **Feature gates**: Conditional compilation with cfg(feature) extensively used - ensure feature is enabled when testing RPC code
2. **Lifetime errors in codec**: If seeing borrow checker issues, verify `Message<'a>` lifetimes match buffer lifetime
3. **Config file required**: Server won't start without `--config` arg - no default config path
4. **Interface bindings**: Empty `server.interfaces` array = no-op server (logs warning but exits successfully)
5. **Port range**: Must be within 49152-65535 per RFC, enforced at runtime
