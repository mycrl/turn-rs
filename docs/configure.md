# Configuration Reference

This document explains every option available in `turn-server.toml`. All keys are written in [TOML](https://toml.io/en/) syntax.

---

## `[server]`

### `server.realm`

-   Type: string
-   Default: `"localhost"`

Realm announced to TURN/STUN clients. See [RFC 5766 §3](https://datatracker.ietf.org/doc/html/rfc5766#section-3) for the formal definition.

### `server.port-range`

-   Type: string in the form `"start..end"`
-   Example: `"49152..65535"`

Inclusive range of relay ports the server is allowed to allocate. Keep the range inside the dynamic port interval (49152–65535) unless you fully control the host.

### `server.max-threads`

-   Type: integer
-   Default: number of logical CPUs

Upper bound for worker threads used by the async runtime.

---

## `[[server.interfaces]]`

You can declare this table multiple times. Every entry describes one listening endpoint.

### `server.interfaces.transport`

-   Type: string enum, required
-   Values: `"udp"` or `"tcp"`

The transport protocol exposed on this interface.

### `server.interfaces.listen`

-   Type: string (`"IP:PORT"`)

Local socket address to bind. Use a specific NIC address when the machine is multi-homed; `0.0.0.0:3478` binds to all IPv4 interfaces.

### `server.interfaces.external`

-   Type: string (`"IP:PORT"`)

Publicly reachable address advertised to clients. Set this to the NAT/public IP when the bound address is not directly reachable.

### `server.interfaces.idle-timeout`

-   Type: integer (seconds)

Maximum idle period before a transport connection is dropped.

### `server.interfaces.mtu`

-   Type: integer (bytes)

Desired MTU for TURN relayed packets.

### `[server.interfaces.ssl]`

-   Keys: `private-key`, `certificate-chain`
-   Type: paths to PEM files (optional)

Provide these fields to enable TLS for the interface. Certificates are loaded via `tokio-rustls` (AWS-LC backend when the `ssl` feature is enabled).

---

## `[api]`

### `api.listen`

-   Type: string (`"IP:PORT"`)
-   Default: `"127.0.0.1:3000"`

Bind address for the management gRPC server.

### `api.timeout`

-   Type: integer (seconds)

Global timeout applied to API handlers.

### `[api.ssl]`

-   Keys: `private-key`, `certificate-chain`
-   Optional TLS configuration for the API endpoint.

> **Security note:** the management gRPC endpoint ships without authentication or TLS. Enable the SSL settings above or terminate TLS behind a proxy before exposing it to untrusted networks.

---

## `[hooks]`

### `hooks.endpoint`

-   Type: string (URL)

Base URL of the external hook service used for dynamic auth and event callbacks.

### `hooks.max-channel-size`

-   Type: integer

Upper bound for buffered hook events.

### `[hooks.ssl]`

-   Keys: `private-key`, `certificate-chain`
-   Optional TLS configuration when communicating with the hook service.

---

## `[log]`

### `log.level`

-   Type: string enum
-   Default: `"info"`
-   Values: `"error" | "warn" | "info" | "debug" | "trace"`

Controls verbosity of the built-in logger.

---

## `[auth]`

### `auth.enable-hooks-auth`

-   Type: boolean
-   Default: `false`

Enable or disable hook-based dynamic authentication.

### `auth.static-auth-secret`

-   Type: string (optional)

Shared secret for TURN REST authentication. When provided, the server skips secret lookups through the hook API.

### `[auth.static-credentials]`

-   Type: table of `username = "password"`

Static user database used before falling back to hook authentication. Populate this map with long-term accounts that should always exist.

---

All settings are hot-reloaded on restart. Keep secrets (private keys, shared tokens) protected with standard filesystem permissions.
