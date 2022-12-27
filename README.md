<!--lint disable no-literal-urls-->
<div align="center">
  <h1>TURN-RS</h1>
</div>
<br/>
<div align="center">
  <strong>TURN Server implemented by ❤️ Rust</strong>
</div>
<div align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/colourful-rtc/turn-rs/cargo-test.yml?branch=main"/>
  <img src="https://img.shields.io/github/license/colourful-rtc/turn-rs"/>
  <img src="https://img.shields.io/github/issues/colourful-rtc/turn-rs"/>
  <img src="https://img.shields.io/github/stars/colourful-rtc/turn-rs"/>
</div>
<br/>
<br/>

A pure rust-implemented turn server, different from coturn, provides a more flexible external control API and provides the same performance and memory footprint.


## Table of contents

* [crates](#crates)
* [building](#building)
* [usage](#usage)


## Crates

* [`stun`], fast and zero cost stun message parser.
* [`turn`], a library for handling turn sessions.
* [`turn-server`], implementation of turn server based on turn library. <sup>([`api`])</sup>

[`api`](https://github.com/mycrl/turn-rs/wiki/External-control-api)
[`stun`]: https://github.com/colourful-rtc/turn-rs/tree/main/stun
[`turn`]: https://github.com/colourful-rtc/turn-rs/tree/main/turn
[`turn-server`]: https://github.com/colourful-rtc/turn-rs/tree/main/turn-server


## Building

### Prerequisites

You need to install the Rust toolchain, if you have already installed it, you can skip it, [Install Rust](https://www.rust-lang.org/tools/install), then get the source code:

```bash
git clone https://github.com/colourful-rtc/turn-rs
```

### Build workspace

Compile the entire workspace in release mode:

```bash
cd turn-rs
cargo build --release
```

After the compilation is complete, you can find the binary file in the "target/release" directory.


## Usage

Show helps:

```bash
turn --help
```

### Command-line arguments
command-line arguments take precedence over environment variables

| values          | default        | env                | tips                       |
|-----------------|----------------|--------------------|----------------------------|
| --realm         | localhost      | TURN_REALM         | turn working relam         |
| --external      | 127.0.0.1:3478 | TURN_EXTERNAL      | turn server public address |
| --bind          | 127.0.0.1:3478 | TURN_BIND          | turn server udp bind port  |
| --nats          | 127.0.0.1:4222 | TURN_NATS          | nats server connection url |
| --nats-token    |                | TURN_NATS_TOKEN    |                            |
| --nats-tls-cert |                | TURN_NATS_TLS_CERT |                            |
| --nats-tls-key  |                | TURN_NATS_TLS_KEY  |                            |
| --threads       |                | TURN_THREADS       | internal thread pool size  |

for sys calls, multithreading does not significantly help to improve IO throughput.

### Simple example

Set envs:

```bash
export TURN_EXTERNAL="127.0.0.1:3478"
export TURN_BIND="127.0.0.1:3478"
```

Or else use command-line arguments:

```bash
turn --bind=127.0.0.1:8080 --external=127.0.0.1:8080
```

### Logs

The server closes log output by default, and the log output level can be set using environment variables:

```bash
export RUST_LOG=<level> // error | warn | info | debug | trace
```


## License

[GPL](./LICENSE)
Copyright (c) 2022 Mr.Panda.
