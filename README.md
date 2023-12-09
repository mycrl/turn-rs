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
  <img src="https://img.shields.io/github/actions/workflow/status/mycrl/turn-rs/tests.yml?branch=main"/>
  <img src="https://img.shields.io/github/license/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/issues/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/stars/mycrl/turn-rs"/>
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
* [benchmark](#benchmark)

## Features

- udp and tcp transport.
- webhooks api. <sup>([`hooks-api`])</sup>
- external controller api. <sup>([`controller-api`])</sup>
- static identity in configuration file.
- only long-term authentication is supported.
- only assign virtual ports.

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


## Benchmark

* CPU: AMD Ryzen 9 7950X 16-Core Processor, RAM: Acer DDR5 5200MHZ 16G x 2, OS: Windows 11 Pro 22H2 - 22621.2428
* turn_relay will use the real network to send from local udp to local udp.

| name                                     | time                          | thrpt                                  |
|------------------------------------------|-------------------------------|----------------------------------------|
| stun_decoder/decoder_channel_bind        | 18.388 ns 18.413 ns 18.439 ns | 5.4549 GiB/s 5.4626 GiB/s 5.4702 GiB/s |
| stun_decoder/decoder_binding             | 17.662 ns 17.672 ns 17.684 ns | 5.0559 GiB/s 5.0592 GiB/s 5.0622 GiB/s |
| turn_router/local_indication_peer        | 35.293 ns 35.319 ns 35.346 ns | *                                      |
| turn_router/peer_indication_local        | 35.384 ns 35.416 ns 35.453 ns | *                                      |
| turn_router/local_channel_data_peer      | 24.644 ns 24.652 ns 24.662 ns | *                                      |
| turn_router/peer_channel_data_local      | 24.622 ns 24.626 ns 24.631 ns | *                                      |
| turn_relay/send_indication_local_to_peer | 42.958 µs 43.040 µs 43.121 µs | *                                      |
| turn_relay/send_indication_peer_to_local | 35.293 ns 35.319 ns 35.346 ns | *                                      |


## License

[MIT](./LICENSE)
Copyright (c) 2022 Mr.Panda.
