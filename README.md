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


## Who uses it?

* [`Psyai`](https://psyai.com)
* [`Faszialespecialist`](https://faszialespecialist.com/)


## Table of contents

* [crates](#crates)
* [building](#building)
* [usage](#usage)
* [benchmark](#benchmark)


## Crates

* [`stun`], fast and zero-cost stun message decoder and encoder. <sup>([`crate`](https://crates.io/crates/faster-stun))</sup>.
* [`turn`], a library for handling turn sessions. <sup>([`crate`](https://crates.io/crates/turn-rs))</sup>.
* [`turn-server`], implementation of turn server based on turn library. <sup>([`api`])</sup>

[`api`]: https://github.com/colourful-rtc/turn-rs/wiki/Controller-API-Reference
[`stun`]: https://github.com/colourful-rtc/turn-rs/tree/main/stun
[`turn`]: https://github.com/colourful-rtc/turn-rs/tree/main/turn
[`turn-server`]: https://github.com/colourful-rtc/turn-rs/tree/main/turn-server


## Building

#### Prerequisites

You need to install the Rust toolchain, if you have already installed it, you can skip it, [Install Rust](https://www.rust-lang.org/tools/install), then get the source code:

```bash
git clone https://github.com/colourful-rtc/turn-rs
```

#### Build workspace

Compile the entire workspace in release mode:

```bash
cd turn-rs
cargo build --release
```

After the compilation is complete, you can find the binary file in the "target/release" directory.


## Usage

Show helps:

```bash
turn-server --help
```

#### Command-line arguments
command-line arguments take precedence over environment variables

| values                | default                | env                      |
|-----------------------|------------------------|--------------------------|
| --realm               | localhost              | TURN_REALM               |
| --external            | 127.0.0.1:3478         | TURN_EXTERNAL            |
| --bind                | 127.0.0.1:3478         | TURN_BIND                |
| --controller-bind     | 127.0.0.1:3000         | TURN_CONTROLLER_BIND     |
| --ext-controller-bind |  http://127.0.0.1:3000 | TURN_EXT_CONTROLLER_BIND |
| --cert-file           |                        | TURN_CERT_FILE           |
| --threads             |                        | TURN_THREADS             |

> for sys calls, multithreading does not significantly help to improve IO throughput.

For detailed documentation, please view: [`Configuration`]

[`Configuration`]: https://github.com/colourful-rtc/turn-rs/wiki/Configuration

#### Simple example

Set envs:

```bash
export TURN_EXTERNAL="127.0.0.1:3478"
export TURN_BIND="127.0.0.1:3478"
```

Or else use command-line arguments:

```bash
turn-server --bind=127.0.0.1:8080 --external=127.0.0.1:8080
```

#### Logs

The server closes log output by default, and the log output level can be set using environment variables:

```bash
export RUST_LOG=<level> // error | warn | info | debug | trace
```


## Benchmark

```
stun_decoder/channel_bind ...[time: 20.606 ns] ...[thrpt: 4.8812 GiB/s]
stun_decoder/binding_request ...[time: 20.862 ns] ...[thrpt: 4.2856 GiB/s]
```


## License

[GPL](./LICENSE)
Copyright (c) 2022 Mr.Panda.
