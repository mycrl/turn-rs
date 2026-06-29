# Migrating from coturn

This guide helps you move an existing [coturn](https://github.com/coturn/coturn)
deployment to turn-rs. It maps the most common `turnserver.conf` options to their
`turn-server.toml` equivalents, explains the conceptual differences, and lists the
coturn features that turn-rs intentionally does **not** implement.

> turn-rs targets the WebRTC use case: long-term credentials, fast UDP/TCP relay,
> and a small, predictable configuration surface. If your coturn setup relies on
> features outside that scope (see [Unsupported features](#unsupported-features)),
> review those sections before switching.

## Before you start

- coturn uses a single flat `turnserver.conf` file (one `key=value` per line, repeatable keys).
- turn-rs uses [TOML](https://toml.io/en/) (`turn-server.toml`), where listeners are
  expressed as repeatable `[[server.interfaces]]` tables.
- See the [Configuration Reference](./configure.md) for every available key.

## Conceptual differences

| Topic           | coturn                                                            | turn-rs                                                                                      |
| --------------- | ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Config format   | `turnserver.conf`, flat `key=value`                               | `turn-server.toml`, structured TOML                                                          |
| Auth mechanisms | Long-term and short-term credentials                              | **Long-term credentials only**                                                               |
| User database   | Static `user=`, plus SQLite / Redis / PostgreSQL / MySQL backends | Static users in config, TURN REST shared secret, or a gRPC **hook** service for dynamic auth |
| Relay ports     | Binds real OS ports between `min-port`/`max-port`                 | Allocates **virtual** ports only; no real system ports are occupied                          |
| TLS             | `tls-listening-port`, plus **DTLS** over UDP                      | TLS via `ssl` on a **TCP** interface; **no DTLS**, UDP has no encryption                     |
| Management      | Telnet/`cli` admin console                                        | Optional **gRPC** management API                                                             |
| Metrics         | Prometheus exporter                                               | Built-in Prometheus exporter (`prometheus` feature)                                          |
| Event callbacks | DB writes / logs                                                  | gRPC **hooks** (allocation, refresh, channel bind, permission, destroy)                      |

## Option mapping

### Listeners, ports and addresses

| coturn (`turnserver.conf`) | turn-rs (`turn-server.toml`)                                 | Notes                                                              |
| -------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------ |
| `listening-port=3478`      | `listen = "0.0.0.0:3478"` in a `[[server.interfaces]]`       | The port is part of the `listen` address.                          |
| `listening-ip=10.0.0.1`    | `listen = "10.0.0.1:3478"`                                   | Bind a specific NIC; use `0.0.0.0` / `[::]` to bind all.           |
| `external-ip=203.0.113.10` | `external = "203.0.113.10:3478"`                             | Public address advertised to clients behind NAT / a load balancer. |
| `min-port` / `max-port`    | `port-range = "49152..65535"` (under `[server]`)             | turn-rs allocates virtual relay ports inside this range.           |
| `no-udp` / `no-tcp`        | Add or omit a `[[server.interfaces]]` with `transport = "…"` | Each transport is one explicit interface entry.                    |
| `listening-ip` (multiple)  | Multiple `[[server.interfaces]]` tables                      | Repeat the table once per NIC/transport.                           |
| `relay-ip`                 | _(no equivalent needed)_                                     | No real relay sockets are bound, so there is nothing to pin.       |

### Realm and authentication

| coturn                                                     | turn-rs                                          | Notes                                                                                                            |
| ---------------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| `realm=example.com`                                        | `realm = "example.com"` (under `[server]`)       | Used for long-term credential hashing.                                                                           |
| `lt-cred-mech`                                             | _(always on)_                                    | turn-rs only supports long-term credentials, so no flag is needed.                                               |
| `user=alice:secret`                                        | `[auth.static-credentials]` → `alice = "secret"` | Static, always-present accounts.                                                                                 |
| `use-auth-secret` + `static-auth-secret=…`                 | `static-auth-secret = "…"` (under `[auth]`)      | TURN REST / time-limited credentials.                                                                            |
| `userdb` / `redis-userdb` / `psql-userdb` / `mysql-userdb` | `enable-hooks-auth = true` + a `[hooks]` service | Replace DB backends with a gRPC hook service that returns passwords. See [hooks](#dynamic-authentication-hooks). |

> Authentication priority in turn-rs: **static credentials → static auth secret → hook `GetPassword`**.

### TLS

| coturn                       | turn-rs                                                                  | Notes                                                            |
| ---------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| `cert=/path/fullchain.pem`   | `certificate-chain = "/path/fullchain.pem"` in `[server.interfaces.ssl]` |                                                                  |
| `pkey=/path/privkey.pem`     | `private-key = "/path/privkey.pem"` in `[server.interfaces.ssl]`         |                                                                  |
| `tls-listening-port=5349`    | A `[[server.interfaces]]` with `transport = "tcp"` + `ssl`               | Enabling `ssl` on a TCP interface turns that interface into TLS. |
| `dtls-listening-port` / DTLS | _(not supported)_                                                        | turn-rs has no DTLS; UDP interfaces cannot be encrypted.         |

### Logging

| coturn                       | turn-rs                                   | Notes                                             |
| ---------------------------- | ----------------------------------------- | ------------------------------------------------- |
| `verbose` / `Verbose`        | `level = "debug"` (under `[log]`)         | Levels: `error`, `warn`, `info`, `debug`.         |
| `log-file=/var/log/turn.log` | `file-directory = "/var/log/turn-server"` | Writes a daily `turn-server-YYYY-MM-DD.log` file. |
| `no-stdout-log`              | `stdout = false` (under `[log]`)          |                                                   |

## Worked example

A typical coturn `turnserver.conf`:

```ini
listening-port=3478
tls-listening-port=5349
listening-ip=0.0.0.0
external-ip=203.0.113.10
min-port=49152
max-port=65535
realm=example.com
lt-cred-mech
user=alice:s3cret
user=bob:hunter2
cert=/etc/turn/fullchain.pem
pkey=/etc/turn/privkey.pem
log-file=/var/log/turnserver.log
```

The equivalent `turn-server.toml`:

```toml
[server]
realm = "example.com"
port-range = "49152..65535"

# Plain UDP listener (TURN default port).
[[server.interfaces]]
transport = "udp"
listen = "0.0.0.0:3478"
external = "203.0.113.10:3478"

# Plain TCP listener.
[[server.interfaces]]
transport = "tcp"
listen = "0.0.0.0:3478"
external = "203.0.113.10:3478"

# TLS listener: a TCP interface with `ssl` becomes TLS (coturn's 5349).
[[server.interfaces]]
transport = "tcp"
listen = "0.0.0.0:5349"
external = "203.0.113.10:5349"

[server.interfaces.ssl]
certificate-chain = "/etc/turn/fullchain.pem"
private-key = "/etc/turn/privkey.pem"

[log]
level = "info"
file-directory = "/var/log/turn-server"

[auth.static-credentials]
alice = "s3cret"
bob = "hunter2"
```

> Note how each coturn listener line becomes its own `[[server.interfaces]]` table,
> and the TLS port is just a TCP interface that carries an `ssl` block.

## Dynamic authentication (hooks)

coturn integrates with SQLite/Redis/PostgreSQL/MySQL user databases. turn-rs has no
built-in database; instead it calls an external **hook service** over gRPC. Enable it
with:

```toml
[auth]
enable-hooks-auth = true

[hooks]
endpoint = "http://127.0.0.1:8080"
```

Your hook service implements `GetPassword` to return the credential for a given
`username` + `realm`, and may receive lifecycle events (`OnAllocatedEvent`,
`OnRefreshEvent`, `OnChannelBindEvent`, `OnCreatePermissionEvent`, `OnDestroyEvent`).
See the protobuf definition in `sdk/protos/server.proto`.

## Management API

coturn ships a Telnet/CLI admin console. turn-rs exposes an optional **gRPC**
management API instead (`GetInfo`, `GetSession`, `GetSessionStatistics`,
`DestroySession`):

```toml
[api]
listen = "127.0.0.1:3000"
```

> The management endpoint has no auth/TLS by default — keep it on a trusted network
> or enable `api.ssl.*`.

## Unsupported features

turn-rs deliberately keeps a small surface. The following coturn capabilities are
**not** available; if you depend on them, plan accordingly:

- **DTLS** (encrypted UDP). Use TLS over TCP instead.
- **Short-term credentials** — only long-term credentials are supported.
- **Built-in user databases** (SQLite/Redis/PostgreSQL/MySQL) — use a gRPC hook service.
- **Per-user / per-realm quotas and bandwidth limiting** (`user-quota`, `total-quota`, `bps-capacity`).
- **`ALTERNATE-SERVER` redirection / load balancing** (RFC 5780 style).
- **Mobility (ICE mobility / `mobility`)**.
- **Telnet/CLI admin console** — replaced by the gRPC API.

## Migration checklist

1. Inventory your `turnserver.conf` and identify any [unsupported features](#unsupported-features).
2. Translate listeners into `[[server.interfaces]]` tables (one per transport/NIC).
3. Move TLS (`cert`/`pkey`) onto a TCP interface via `[server.interfaces.ssl]`.
4. Recreate static users under `[auth.static-credentials]`, or wire up a hook service for dynamic auth.
5. Set `realm`, `port-range`, and logging to match your current behavior.
6. Validate with a WebRTC client (see the [WebRTC demo](https://mycrl.github.io/turn-rs/demo)) before cutting over production traffic.
