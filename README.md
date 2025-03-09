<!--lint disable no-literal-urls-->
<div align="center">
  <img src="./logo.svg" width="200px"/>
</div>
<br/>
<div align="center">
  <strong>TURN Server implemented by ❤️ Rust</strong>
</div>
<div align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/mycrl/turn-rs/tests.yml?branch=main"/>
  <img src="https://img.shields.io/github/license/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/issues/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/stars/mycrl/turn-rs"/>
</div>
<br/>
<br/>

A pure Rust implementation of a forwarding server that takes advantage of the memory and concurrency security provided by Rust, with single-threaded decoding speeds up to 5Gib/s and forwarding latency of less than 35 microseconds（One complete local network send/receive between A and B delay）. The project is more focused on the core business , do not need to access the complex configuration project , almost out of the box.

This is a very lightweight implementation, and turn-rs will get your data flowing quickly if you only start the basic functionality, and while it uses pre-allocated memory in many places to cope with bursty performance, it generally performs well (it delivers very high-speed forwarding performance on my Raspberry Pi 4 as well as still performs well in the face of a large number of clients).

If you only need a pure turn server to cope with WebRTC business and require excellent forwarding performance, the current project will satisfy you.

## Differences with coturn?

First of all, I remain in awe and respect for coturn, which is a much more mature implementation and has very comprehensive support for a wide range of features.

However, turn-rs is not a simple duplicate implementation, and this project is not a blind “RIIR”. Because turn server is currently the largest use of the scene or WebRTC, for WebRTC business, many features are not too much necessary, so keep it simple and fast is the best choice.

##### "Better performance"

Because turn-rs only focuses on the core business, it removes a lot of features that are almost less commonly used in WebRTC scenarios, resulting in better performance, both in terms of throughput and memory performance.

##### "Database storage is not supported"

I don't think turn servers should be concerned about user information, just do their essential work, it's better to leave the hosting and storing of user information to other services, and interacting with databases adds complexity. turn-rs communicates with external services through http hooks, which can be more flexible in deciding how to deal with it based on their own business situation.

##### "No transport layer encryption"

Unlike coturn, which provides various transport layer encryption, turn-rs does not provide any transport layer encryption. Currently turn clients that support encryption are relatively rare, and there is minimal benefit to the turn server in providing transport layer encryption, since for WebRTC the transport data is already encrypted.

##### "Only allow turn-rs as transit address"

Some clients currently use local addresses for the turn server to create bindings and permissions under certain NAT types, coturn supports this behaviour. However, turn-rs does not allow this behaviour, any client must use the turn server's transit address to communicate, which provides help for clients to hide their IP addresses.

## Table of contents

-   [features](#features)
-   [usage](#usage)
    -   [docker](#docker)
    -   [linux service](#linux-service)
-   [building](#building)
-   [document](./docs)
    -   [install](./docs/install.md)
    -   [build](./docs/build.md)
    -   [start the server](./docs/start-the-server.md)
    -   [configure](./docs/configure.md)
    -   [api](./docs/rest-api.md)
-   [driver](./drivers) - ([crates.io](https://crates.io/crates/turn-driver)) Integration with turn-rs server is easy with rust.

## Features

-   Prometheus metrics exporter.
-   Only long-term credential mechanisms are used.
-   Static authentication lists can be used in configuration files.
-   Only virtual ports are always allocated and no real system ports are occupied.
-   The transport layer supports TCP and UDP protocols, and supports binding multiple network cards or interfaces.
-   The REST API can be used so that the turn server can proactively notify the external service of events and use external authentication mechanisms, and the external can also proactively control the turn server and manage the session.

#### RFC

-   [RFC 3489](https://datatracker.ietf.org/doc/html/rfc3489) - "classic" STUN
-   [RFC 5389](https://datatracker.ietf.org/doc/html/rfc5389) - base "new" STUN specs
-   [RFC 5769](https://datatracker.ietf.org/doc/html/rfc5769) - test vectors for STUN protocol testing
-   [RFC 5766](https://datatracker.ietf.org/doc/html/rfc5766) - base TURN specs
-   [RFC 6062](https://datatracker.ietf.org/doc/html/rfc6062) - TCP relaying TURN extension
-   [RFC 6156](https://datatracker.ietf.org/doc/html/rfc6156) - IPv6 extension for TURN
-   TURN REST API (http://tools.ietf.org/html/draft-uberti-behave-turn-rest-00)

## Usage

First, Get the compiled binaries from [github release](https://github.com/mycrl/turn-rs/releases).

Start with configuration file:

```bash
turn-server --config=/etc/turn-server/config.toml
```

Please check the example configuration file for details: [turn-server.toml](./turn-server.toml)

#### Docker

```bash
docker pull ghcr.io/mycrl/turn-server
```

The custom configuration file overrides the `/etc/turn-server/config.toml` path inside the image through `-v`.

#### Linux service

```
./install-service.sh
```

This will compile the project and install and start the service.

## Building

#### Prerequisites

You need to install the Rust toolchain, if you have already installed it, you can skip it, [Install Rust](https://www.rust-lang.org/tools/install), then get the source code:

```bash
git clone https://github.com/mycrl/turn-rs
```

#### Build workspace

Compile the entire workspace in release mode:

```bash
cd turn-rs
cargo build --release
```

#### Features

-   `udp` - (enabled by default) Enables UDP transport layer support.
-   `tcp` - Enables TCP transport layer support.
-   `api` - Enable the HTTP REST API server feature.
-   `prometheus` - Enable prometheus indicator support.

No features are enabled by default and need to be turned on by manual specification.

```bash
cargo build --release --all-features
```

After the compilation is complete, you can find the binary file in the `"target/release"` directory.

## License

[LGPL](./LICENSE)
Copyright (c) 2022 Mr.Panda.
