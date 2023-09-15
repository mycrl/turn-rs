<!--lint disable no-literal-urls-->
<div align="right">
  <a href="./README.CN.md">简体中文</a>
  /
  <a href="./README.md">English</a>
</div>
<div align="center">
  <h1>TURN-RS</h1>
</div>
<br/>
<div align="center">
  <strong>TURN Server implemented by ❤️ Rust</strong>
</div>
<div align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/mycrl/turn-rs/release.yml?branch=main"/>
  <img src="https://img.shields.io/github/license/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/issues/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/stars/mycrl/turn-rs"/>
</div>
<br/>
<br/>

A pure rust-implemented turn server, different from coturn, provides a more flexible external control API and provides the same performance and memory footprint, this project is most compatible with the scenario of using stun/turn server in webrtc.


## Who uses it?

* [`Psyai`](https://psyai.com)
* [`Faszialespecialist`](https://faszialespecialist.com/)


## Table of contents

* [features](#features)
* [usage](#usage)
  * [docker](#docker)  
  * [linux service](#linux-service)
* [building](#building)


## Features

- udp and tcp transport.
- webhooks api. <sup>([`hooks-api`])</sup>
- external controller api. <sup>([`controller-api`])</sup>
- static identity in configuration file.
- only long-term authentication is supported.
- virtual port support. <sup>(`allocate request does not allocate real udp ports`)</sup>

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


#### Docker

```bash
docker pull quasipaa/turn-server
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
