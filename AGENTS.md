# AGENTS.md

This document targets two audiences:

1) LLMs/agents: quickly understand project structure, entry points, run flow, configuration, and risks.
2) Human developers/operators: follow the steps to build, configure, start, and verify the service.

## Project Overview

turn-rs is a TURN/STUN server implemented in Rust for WebRTC NAT traversal and media relay. It focuses on high performance and low configuration cost, and provides optional gRPC management APIs, Prometheus metrics, and Hook callbacks.

## Core Capabilities

- TURN/STUN protocol support with TCP/UDP transport.
- Long-term credential mechanism with static users and hook-based dynamic auth.
- Optional gRPC management API and Prometheus metrics exporter.
- Multi-interface listeners and external address announcement.

## Key Entry Points and Directories

- Runtime entry: [src/main.rs](src/main.rs)
- Library entry: [src/lib.rs](src/lib.rs)
- Service and session logic: [src/service](src/service)
- Protocol handling and codecs: [src/codec](src/codec)
- Transport layer: [src/server/transport](src/server/transport)
- Config and logging: [src/config.rs](src/config.rs), [src/logger.rs](src/logger.rs)
- gRPC API: [protos/protobufs/server.proto](protos/protobufs/server.proto)
- Sample config: [turn-server.toml](turn-server.toml)
- Docs entry: [docs/README.md](docs/README.md)

## Source Code Architecture (Modules and Responsibilities)

This section explains how the server is organized internally and how the main data flow works.

### High-level runtime flow

1) [src/main.rs](src/main.rs) loads config, initializes logging, and builds the Tokio runtime.

2) [src/lib.rs](src/lib.rs) `start_server()` constructs `Statistics`, a `Handler`, and a `Service`, then spawns:

- transport servers (UDP/TCP) via [src/server](src/server)
- optional Prometheus exporter via [src/prometheus.rs](src/prometheus.rs)
- optional gRPC API via [src/api.rs](src/api.rs)

### Core modules

1) [src/service](src/service): TURN service core, shared state, and routing glue.

- `Service` holds realm, interfaces, session manager, and handler, and creates per-connection routers.
- `ServiceHandler` defines the hooks the protocol layer uses for auth and lifecycle callbacks.
- [src/service/routing.rs](src/service/routing.rs) parses STUN/TURN messages and dispatches by method.

2) [src/service/session](src/service/session): Session state, allocation, permissions, and channel bindings.

- `Identifier` (source + interface) is the primary session key.
- `SessionManager` owns sessions, port mappings, permissions, and channel relay tables.
- `Session` tracks authentication state, nonce, allocated port, channels, permissions, and expiry.
- [src/service/session/ports.rs](src/service/session/ports.rs) provides `PortAllocator` and `PortRange`.

3) [src/server](src/server): Transport orchestration and cross-protocol forwarding.

- Spawns TCP/UDP listeners per configured interface.
- `Exchanger` maps interface address to internal channels for forwarding packets between sockets.
- Uses the `Server` trait to share TCP/UDP accept/read/write logic.

4) [src/server/transport](src/server/transport): Transport abstraction and server loop.

- `Server::start` binds sockets, spawns per-connection tasks, routes packets, and handles idle timeout.
- `Transport` (TCP/UDP) drives stats reporting and channel-data padding rules for TCP.

5) [src/handler.rs](src/handler.rs): Implements `ServiceHandler`.

- Auth flow: static credentials -> static auth secret -> optional Hook `GetPassword`.
- Lifecycle events: allocation, channel bind, permission create, refresh, destroy (sent to Hook service when enabled).

6) [src/api.rs](src/api.rs): gRPC management API and Hook client implementation.

- `TurnService` exposes GetInfo/GetSession/GetSessionStatistics/DestroySession.
- `RpcHooksService` maintains a client + buffered event channel to the external Hook service.

7) [src/codec](src/codec): STUN/TURN codec and crypto.

- Decoder differentiates STUN messages vs. ChannelData.
- Message encoder/decoder handles attributes, integrity, and fingerprint.
- `crypto` contains HMAC and password derivation helpers.

8) [src/statistics.rs](src/statistics.rs): Per-session counters and reporting.

- `StatisticsReporter` aggregates per-session bytes/packets and error counts.
- Integrates with Prometheus metrics when enabled.

9) [src/prometheus.rs](src/prometheus.rs): HTTP metrics endpoint.

- Exposes `/metrics`, tracks global + per-transport counts and allocated sessions.

### Design notes and key decisions

- Long-term credentials are the primary auth model; Hook auth is optional and pluggable.
- Port allocation is a pre-sized bitset allocator for fast random relay port selection.
- Session tables are pre-sized HashMaps for performance under load.
- Router validates peer addresses against local interfaces by default to reduce abuse risk.
- Transport loop is unified with a trait, but TCP/UDP sockets have their own implementations.

## Quick Start

### Option 1: Release Binary

1) Download the binary from GitHub Releases for your platform.
2) Prepare a config file (see [turn-server.toml](turn-server.toml)).
3) Start the server:

```bash
turn-server --config ./turn-server.toml
```

### Option 2: Build From Source

Install the Rust toolchain, then run in the project root:

```bash
cargo build --release
```

The binary will be in the target/release directory.

## Configuration

The config file uses TOML. Full reference: [docs/configure.md](docs/configure.md).

### Configuration capabilities (feature-oriented)

- `server.*` defines reachability and transport surfaces: `server.interfaces` supports multi-NIC and multi-transport (`udp`/`tcp`) listeners, `listen` binds the local address, and `external` advertises the public address to clients behind NAT or load balancers. `server.port-range` limits relay port allocation, `server.max-threads` caps runtime workers, and `server.realm` is a key input for long-term credential auth.
- `server.interfaces.idle-timeout` and `server.interfaces.mtu` protect connection lifecycle and path stability: the former reclaims idle resources, the latter reduces fragmentation risk when relaying. The MTU setting applies to UDP transport only.
- TLS is enabled per surface: data plane via `server.interfaces.ssl.*` (TCP transport only), management plane via `api.ssl.*`, and metrics plane via `prometheus.ssl.*`. This lets you secure exposed endpoints while keeping internal ones lightweight.
- Auth strategy is defined by `auth.*`: `auth.static-credentials` provides local static users, `auth.static-auth-secret` enables TURN REST-style shared secrets; for dynamic auth, combine `auth.enable-hooks-auth` with `hooks.*` so an external Hook service can provide passwords and handle session events. Priority is static users first, then shared secret, then Hooks.
- `hooks.*` enables external integrations for dynamic auth and lifecycle callbacks (allocation, refresh, destroy, and more). `hooks.max-channel-size` and `hooks.timeout` control backpressure and timeouts so Hooks do not impact the main data path.
- `api.*` enables the gRPC management interface for querying server info, session state, statistics, and destroying sessions.
- `prometheus.*` exposes Prometheus metrics (requires the `prometheus` feature at build time).
- `log.*` controls observability output: `log.level` sets verbosity, `log.stdout` fits container or systemd aggregation, and `log.file-directory` enables local log retention.

## Start the Server

Basic command:

```bash
turn-server --config ./turn-server.toml
```

For Linux systemd service, see [docs/start-the-server.md](docs/start-the-server.md).

## Docker

Docker image is published on GitHub Packages. Pull and mount your config:

```bash
docker pull ghcr.io/mycrl/turn-server:latest
# Override the default config path inside the container
# Default path: /etc/turn-server/config.toml
```

See [docs/install.md](docs/install.md) for details.

## Build Features (Optional)

You can reduce the binary by compiling with specific features:

- udp: UDP transport (default on)
- tcp: TCP transport
- ssl: TLS support
- api: gRPC management API
- prometheus: metrics exporter

Example:

```bash
cargo build --release --no-default-features --features udp,tcp
```

## API and Hooks

The following section explains how these capabilities work and what they provide, for readers unfamiliar with TURN ecosystems.

### gRPC Management API (server exposes to clients)

Purpose: allow external systems to query server status, inspect sessions, collect stats, and destroy sessions.

Protocol and fields: [protos/protobufs/server.proto](protos/protobufs/server.proto). Core RPCs:

- `GetInfo`: returns software info, uptime, listening interfaces, port capacity, and allocated ports.
- `GetSession`: query a session by `id`, returns username, permissions, channels, allocated port, and expiry.
- `GetSessionStatistics`: per-session bytes/packets and error packet counts.
- `DestroySession`: terminate a session by `id`.

Enablement and security:

- This endpoint has no TLS or auth by default. If exposed beyond a trusted network, enable `api.ssl.*`.
- Bind address is configured by `api.listen` (default 127.0.0.1:3000).
- Timeouts are configured by `api.timeout`.

### Hook Service (server calls external system)

Purpose: dynamic authentication and event callbacks. At specific moments, turn-rs calls the external Hook service. The external service can decide whether to allow access and can record or integrate lifecycle events.

Protocol and fields: [protos/protobufs/server.proto](protos/protobufs/server.proto). Two categories:

1) Dynamic authentication

- `GetPassword`: server asks for the password used to compute TURN message integrity.
- Request includes `username`, `realm`, and `algorithm` (`MD5` or `SHA256`).
- Response returns `password` as bytes.

2) Event callbacks

- `OnAllocatedEvent`: relay allocation completed.
- `OnChannelBindEvent`: channel bound.
- `OnCreatePermissionEvent`: permission created.
- `OnRefreshEvent`: allocation refresh/extend.
- `OnDestroyEvent`: session destroyed.

Enablement and behavior:

- Hook address is configured by `hooks.endpoint`, with TLS via `hooks.ssl.*`.
- `auth.static-credentials` takes priority over Hook auth.
- If `auth.static-auth-secret` is configured, the server skips Hook password lookups.
- `hooks.timeout` controls request timeouts; `hooks.max-channel-size` limits event buffering.

Typical use cases:

- Integrate with your account system for dynamic auth (temporary tickets, internal SSO).
- Record session lifecycle metrics for auditing or risk analysis.

## Logging and Observability

- log.level controls log verbosity.
- log.stdout enables or disables stdout logs.
- log.file-directory writes logs to a daily file.
- prometheus.listen enables the metrics endpoint (requires `prometheus` feature).

## Security Notes

- gRPC management endpoint has no auth/TLS by default; enable `api.ssl.*` or keep it in a trusted network.
- Protect certificates, private keys, and shared secrets with filesystem permissions.

## Tests and Benchmarks

- Unit/integration tests:

```bash
cargo test
```

- Benchmarks (optional):

```bash
cargo bench
```

## Common Operations

- Update config: edit [turn-server.toml](turn-server.toml) and restart the service.
- Multi-interface: add multiple entries under server.interfaces with distinct listen/external.
- NAT environment: set external to a public reachable address so clients receive correct candidates.

## Suitable Scenarios

- WebRTC TURN relay
- High-throughput media forwarding with stable long-lived connections

## Not Suitable Scenarios

- Full coturn feature parity
- Complex auth systems without a deployable Hook service

## Maintenance Notes (for agents/automation)

- Entry logic is in [src/main.rs](src/main.rs) and [src/service](src/service).
- Config structs are in [src/config.rs](src/config.rs); update [docs/configure.md](docs/configure.md) and [turn-server.toml](turn-server.toml) when adding fields.
- gRPC API changes must update [protos/protobufs/server.proto](protos/protobufs/server.proto) and any generated code.
- Transport changes are in [src/server/transport](src/server/transport).
