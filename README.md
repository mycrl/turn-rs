<!--lint disable no-literal-urls-->
<div align="center">
  <img 
    alt="turn-rs"
    src="./logo.svg" 
    width="200px"
  />
</div>
<br/>
<div align="center">
  <strong>TURN Server implemented by ❤️ Rust</strong>
</div>
<div align="center">
  <img src="https://img.shields.io/github/workflow/status/mycrl/turn-rs/cargo-test"/>
  <img src="https://img.shields.io/github/license/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/issues/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/stars/mycrl/turn-rs"/>
</div>
<br/>
<br/>

A pure rust-implemented turn server, different from coturn, provides a more flexible external control API and provides the same performance and memory footprint.


## Table of contents

* [building](#building)
  * [prerequisites](#prerequisites)
  * [build workspace](#build-workspace)
  * [docker compose](#docker-compose)


## Building

### Prerequisites

You need to install the Rust toolchain, if you have already installed it, you can skip it, [Install Rust](https://www.rust-lang.org/tools/install), then get the source code:

```bash
git clone https://github.com/mycrl/turn-rs
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
| --listening     | 127.0.0.1:3478 | TURN_BIND          | turn server udp bind port  |
| --nats          | 127.0.0.1:4222 | TURN_NATS          | nats server connection url |
| --nats_token    |                | TURN_NATS_TOKEN    |                            |
| --nats_tls_cert |                | TURN_NATS_TLS_CERT |                            |
| --nats_tls_key  |                | TURN_NATS_TLS_KEY  |                            |
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
turn --listening=127.0.0.1:8080 --external=127.0.0.1:8080
```

### Logs

The server closes log output by default, and the log output level can be set using environment variables:

```bash
export RUST_LOG=<level> // error | warn | info | debug | trace
```

## External control api

> Public response
* error `{Option<String>}` - error info.
* data `{Option<T>}` - response data.

> Auth - `turn.auth`

Request:
* addr `{SocketAddr}` - udp client session address.
* realm `{String}` - turn server realm.
* username `{String}` - session username.

Response:
* password `{String}` - session password.
* group `{u32}` - session group id.

> Close - `turn.<realm>.close`

Request:
* addr `{String}` - session address.

> Get state - `turn.<realm>.state`

Response:
* capacity `{Number}` - turn port capacity.
* users `{Array<[String, Array<String>]>}` - turn allocated user list.
* len `{Number}` - users size.

> Get node - `turn.<realm>.node`

Request:
* username `{String}`.

Response:
* channels `{Array<Number>}` - turn allocated channel numbers.
* ports `{Array<Number>}` - turn allocated port numbers.
* timer `{Number}` - allocated time.
* lifetime `{Number}` - allocate lifetime.


## Code style

The coding style of this project may not conform to the community style or the habits of most people, but it conforms to my own style. I have paranoid requirements for the code format, I know this is a bad habit, and the current project is also independently developed and maintained by me. If you have more suggestions, you can tell me.


## License

[GPL](./LICENSE)
Copyright (c) 2022 Mr.Panda.
