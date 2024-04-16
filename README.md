<!--lint disable no-literal-urls-->
<div align="right">
  <a href="./README.CN.md">简体中文</a>
  /
  <a href="./README.md">English</a>
</div>
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
<div align="center">
  <sup>RFC: https://datatracker.ietf.org/doc/html/rfc8656</sup>
</div>
<br/>
<br/>

A pure Rust implementation of the turn server. Compared with coturn, the advantage is to provide better performance. Single-threaded decoding speed up to 5Gib/s, forwarding delay is less than 35 microseconds. However, it does not provide as rich as coturn feature support, this project is more focused on the core business, do not need to touch the complex configuration items, almost out of the box.

## How do I choose?

#### turn-rs

If you are not familiar with coturn configuration items and are annoyed by the complexity of coturn configuration items, then you should use this project, and similarly, if you want better performance performance and lower memory footprint, you can also use this project. turn-rs configuration is easy, and the external api is very simple, and is good enough for core business support.

#### coturn

If you have extensive standard support requirements for turn servers and need more integrated services and ecological support, then you should choose coturn.

## Who uses it?

* [`Psyai`](https://psyai.com) <sup>(turn-rs has been in service for more than a year without any faults or downtime.)</sup>
* [`Faszialespecialist`](https://faszialespecialist.com/)


## Table of contents

* [features](#features)
* [components](#components)
* [building](#building)


## Features

- Only long-term authentication mechanisms are supported.
- Static authentication lists can be used in configuration files.
- Only virtual ports are always allocated and no real system ports are occupied.
- The transport layer supports tcp and udp protocols, and supports binding multiple network cards or interfaces.
- Provides a simple command line tool to manage and monitor the turn server through the command line tool graphical interface. <sup>([`turn-cli`])</sup>
- With a load balanced server, you can allow users to reach your turn server quickly with the best line. <sup>([`turn-balance`])</sup>
- The grpc interface can be used so that the turn server can proactively notify the external service of events and use external authentication mechanisms, and the external can also proactively control the turn server and manage the session. <sup>([`protos`])</sup>

[`turn-balance`]: ./turn-balance
[`turn-cli`]: ./cli
[`protos`]: ./protos


## Components

* [turn server](./turn-server) - A pure Rust implementation of the turn server.
* [turn balance](./turn-balance) - A simple distributed load balancing service.
* [turn cli](./cli) - A simple turn server command line monitoring tool.
* [turn driver](./driver) - A turn server driver for rust.


## Building

#### Prerequisites

You need to install the Rust toolchain, if you have already installed it, you can skip it, [Install Rust](https://www.rust-lang.org/tools/install), then get the source code:

```bash
git clone https://github.com/mycrl/turn-rs
```

##### Protoc

Because all internal communication is based on `protobuf`, and the `prost-build` required to compile `.proto` files needs to rely on `protoc`, so please install protoc into your environment, to install `protoc` please see the [protobuf installation](https://github.com/protocolbuffers/protobuf#protobuf-compiler-installation) instructions.

#### Build workspace

Compile the entire workspace in release mode:

```bash
cd turn-rs
cargo build --release
```

After the compilation is complete, you can find the binary file in the `"target/release"` directory.  

> turn server uses mimalloc memory allocator by default, and the third party memory allocator is not very friendly in terms of memory reclaim speed and memory usage for performance consideration, if you feel mindful of this, you can use `cargo build --release --features system_allocator` option to switch to the platform's default memory allocator at compile time.


## License

[GPL3.0](./LICENSE)
Copyright (c) 2022 Mr.Panda.
