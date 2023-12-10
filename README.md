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

A pure rust-implemented turn server, different from coturn, provides a more flexible external control API and provides the same performance and memory footprint, this project is most compatible with the scenario of using stun/turn server in webrtc.


## Who uses it?

* [`Psyai`](https://psyai.com) <sup>(turn-rs has been in service for more than a year without any faults or downtime.)</sup>
* [`Faszialespecialist`](https://faszialespecialist.com/)


## Table of contents

* [features](#features)
* [usage](#usage)
  * [docker](#docker)  
  * [linux service](#linux-service)
* [building](#building)


## Features

- The transport layer supports tcp and udp protocols, and supports binding multiple network cards or interfaces..
- You can use the WebHooks api, and the turn server can actively notify external services of some events and use external authentication mechanisms. <sup>([`hooks-api`])</sup>
- External control API, external parties can actively control the turn server and manage sessions.. <sup>([`controller-api`])</sup>
- Static authentication lists can be used in configuration files.
- Only long-term authentication mechanisms are supported.
- Only virtual ports are always allocated and no real system ports are occupied.

[`controller-api`]: https://github.com/mycrl/turn-rs/wiki/Controller-API-Reference
[`hooks-api`]: https://github.com/mycrl/turn-rs/wiki/Hooks-API-Reference


## Usage

> The versions on crates.io and docker may be very outdated. It is recommended to compile directly from the github source code.

```bash
cargo install turn-server
```

Start with configuration file:

```bash
turn-server --config=/etc/turn_server/config.toml
```

Please check the example configuration file for details: [turn_server.toml](./turn_server.toml)  
Please see the [wiki](https://github.com/mycrl/turn-rs/wiki/Configuration) for a description of the configuration file.


#### Docker

```bash
// docker hub
docker pull quasipaa/turn-server
// github packages
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

After the compilation is complete, you can find the binary file in the "target/release" directory.


## License

[MIT](./LICENSE)
Copyright (c) 2022 Mr.Panda.
