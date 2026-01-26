<!--lint disable no-literal-urls-->
<div align="center">
    <img src="./logo.svg" width="200px"/>
</div>
<br/>
<div align="center">
    <strong>TURN Server implemented by ❤️ Rust</strong>
    <p>The TURN Server is a VoIP media traffic NAT traversal server and gateway.</p>
</div>
<div align="center">
    <img src="https://img.shields.io/github/actions/workflow/status/mycrl/turn-rs/tests.yml?branch=main&style=flat-square"/>
    <img src="https://img.shields.io/crates/v/turn-server?style=flat-square"/>
    <img src="https://img.shields.io/docsrs/turn-server?style=flat-square"/>
    <img src="https://img.shields.io/github/license/mycrl/turn-rs?style=flat-square"/>
    <img src="https://img.shields.io/github/issues/mycrl/turn-rs?style=flat-square"/>
    <img src="https://img.shields.io/github/stars/mycrl/turn-rs?style=flat-square"/>
</div>
<div align="center">
    <a href="https://zdoc.app/de/mycrl/turn-rs">Deutsch</a> | 
    <a href="https://zdoc.app/es/mycrl/turn-rs">Español</a> | 
    <a href="https://zdoc.app/fr/mycrl/turn-rs">français</a> | 
    <a href="https://zdoc.app/ja/mycrl/turn-rs">日本語</a> | 
    <a href="https://zdoc.app/ko/mycrl/turn-rs">한국어</a> | 
    <a href="https://zdoc.app/pt/mycrl/turn-rs">Português</a> | 
    <a href="https://zdoc.app/ru/mycrl/turn-rs">Русский</a> | 
    <a href="https://zdoc.app/zh/mycrl/turn-rs">中文</a>
</div>

---

A pure Rust implementation of a forwarding server leverages Rust's memory and concurrency safety to process 40 million channel data forwarding messages and 600,000 allocation requests per second within a single thread (excluding network stack overhead). Forwarding latency remains below 35 microseconds (equivalent to a complete local network send/receive delay between points A and B). This project prioritizes core functionality, requiring minimal configuration for use and offering near-out-of-the-box usability.

This is a very lightweight implementation, and turn-rs will get your data flowing quickly if you only start the basic functionality, and while it uses pre-allocated memory in many places to cope with bursty performance, it generally performs well (it delivers very high-speed forwarding performance on my Raspberry Pi 4 as well as still performs well in the face of a large number of clients).

If you only need a pure turn server to cope with WebRTC business and require excellent forwarding performance, the current project will satisfy you.

## Differences with coturn?

First of all, I remain in awe and respect for coturn, which is a much more mature implementation and has very comprehensive support for a wide range of features.

However, turn-rs is not a simple duplicate implementation, and this project is not a blind “RIIR”. Because turn server is currently the largest use of the scene or WebRTC, for WebRTC business, many features are not too much necessary, so keep it simple and fast is the best choice.

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
    -   [api](./protos/protobufs/server.proto)

## Features

-   Only long-term credential mechanisms are used.
-   Static authentication lists can be used in configuration files.
-   Only virtual ports are always allocated and no real system ports are occupied.
-   The transport layer supports TCP and UDP protocols, and supports binding multiple network cards or interfaces.
-   The GRPC API can be used so that the turn server can proactively notify the external service of events, and the external can also proactively control the turn server and manage the session.

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

If you don't need a particular feature, you can reduce the package size by enabling only the features you require.

-   `udp` - (enabled by default) Enables UDP transport layer support.
-   `tcp` - Enables TCP transport layer support.
-   `ssl` - Enable SSL encryption support.
-   `api` - Enable the gRPC api server feature.
-   `prometheus` - Enable prometheus support.

All features are enabled by default.

```bash
cargo build --release
```

After the compilation is complete, you can find the binary file in the `"target/release"` directory.

## License

[MIT](./LICENSE)
Copyright (c) 2022 Mr.Panda.
