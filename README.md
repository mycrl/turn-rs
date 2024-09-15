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

A pure Rust implementation of the turn server. Compared with coturn, the advantage is to provide better performance. Single-threaded decoding speed up to 5Gib/s, forwarding delay is less than 35 microseconds. However, it does not provide as rich as coturn feature support, this project is more focused on the core business, do not need to touch the complex configuration items, almost out of the box.

## How do I choose?

#### turn-rs

If you are not familiar with coturn configuration items and are annoyed by the complexity of coturn configuration items, then you should use this project, and similarly, if you want better performance performance and lower memory footprint, you can also use this project. turn-rs configuration is easy, and the external api is very simple, and is good enough for core business support.

#### coturn

If you have extensive standard support requirements for turn servers and need more integrated services and ecological support, then you should choose coturn.


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

> turn-rs is based on WebRTC usage scenarios, so when providing support for a typical WebRTC session, most of the features that a turn server should have are already supported, and if there are no supported features, most of the time a similar mechanism is provided.As for why we don't follow RFCs, turn-rs has its own considerations, such as the fact that some RFCs are more complex to implement, but can be used in very few scenarios, or that the RFCs themselves are not well developed, and the technical specifications are rather old.

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


## License

[GPL3.0](./LICENSE)
Copyright (c) 2022 Mr.Panda.
