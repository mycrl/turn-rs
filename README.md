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

A pure Rust implementation of a forwarding server that takes advantage of the memory and concurrency security provided by Rust, with single-threaded decoding speeds up to 5Gib/s and forwarding latency of less than 35 microseconds. The project is more focused on the core business , do not need to access the complex configuration project , almost out of the box .

## Differences with coturn?

First of all, I remain in awe and respect for coturn, which is a much more mature implementation and has very comprehensive support for a wide range of features.

However, turn-rs is not a simple duplicate implementation, and this project is not a blind “RIIR”. Because turn server is currently the largest use of the scene or WebRTC, for WebRTC business, many features are not too much necessary, so keep it simple and fast is the best choice.

##### "Better performance"

Because turn-rs only focuses on the core business, it removes a lot of features that are almost less commonly used in WebRTC scenarios, resulting in better performance, both in terms of throughput and memory performance.

##### "Database storage is not supported"

I don't think turn servers should be concerned about user information, just do their essential work, it's better to leave the hosting and storing of user information to other services, and interacting with databases adds complexity. turn-rs communicates with external services through http hooks, which can be more flexible in deciding how to deal with it based on their own business situation.

##### "No transport layer encryption"

This is an obvious drawback, unlike coturn which provides various transport layer encryption, turn-rs doesn't provide any transport layer encryption, but turn-rs mainly serves WebRTC business, WebRTC comes with transport layer encryption, and the packets transmitted in turn are already encrypted, so in order to reduce the overhead, turn -rs does not provide transport layer encryption.

## Table of contents

* [features](#features)
* [usage](#usage)
  * [docker](#docker)  
  * [linux service](#linux-service)
* [building](#building)
* [document](./docs)
  * [install](./docs/install.md)
  * [build](./docs/build.md)
  * [start the server](./docs/start-the-server.md)
  * [configure](./docs/configure.md)
  * [rest api](./docs/rest-api.md)
  * [http hooks](./docs/http-hooks.md)
* [driver](./drivers)

## Features

> Compared with the standard RFC, some restrictions are added: turn-rs only allows operations on addresses in the current server interface, and does not allow clients to forward data or create bindings to addresses that are not in the turn server address list. Fortunately, most turn clients currently follow this convention, such as Firefox and Chrome's WebRTC implementation.

- Prometheus metrics exporter.
- Only long-term authentication mechanisms are used.
- Static authentication lists can be used in configuration files.
- Only virtual ports are always allocated and no real system ports are occupied.
- The transport layer supports tcp and udp protocols, and supports binding multiple network cards or interfaces.
- The REST API can be used so that the turn server can proactively notify the external service of events and use external authentication mechanisms, and the external can also proactively control the turn server and manage the session.

#### RFC

* [RFC 3489](https://datatracker.ietf.org/doc/html/rfc3489) - "classic" STUN
* [RFC 5389](https://datatracker.ietf.org/doc/html/rfc5389) - base "new" STUN specs
* [RFC 5769](https://datatracker.ietf.org/doc/html/rfc5769) - test vectors for STUN protocol testing
* [RFC 5766](https://datatracker.ietf.org/doc/html/rfc5766) - base TURN specs
* [RFC 6062](https://datatracker.ietf.org/doc/html/rfc6062) - TCP relaying TURN extension
* [RFC 6156](https://datatracker.ietf.org/doc/html/rfc6156) - IPv6 extension for TURN
* TURN REST API (http://tools.ietf.org/html/draft-uberti-behave-turn-rest-00)

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

After the compilation is complete, you can find the binary file in the `"target/release"` directory.

Prometheus metrics exporter is not enabled by default, use the `prometheus` feature flag if you need to enable it:

```bash
cargo build --release --features prometheus
```

## License

[LGPL](./LICENSE)
Copyright (c) 2022 Mr.Panda.
