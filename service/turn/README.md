<!--lint disable no-literal-urls-->
<br/>
<br/>
<div align="center">
  <img 
    alt="psyai-net"
    src="../logo.jpg" 
    width="70px"
  />
</div>
<br/>
<div align="center">
  <strong>Cloud Render - TURN</strong>
</div>
<br/>


The WebRTC TURN server is implemented according to RFC 5766 and adds support for session grouping in the specification. The difference from the common open-source solution is that it only supports the UDP protocol and only supports long-term credentials. The credentials are obtained in real-time through RPC.


## Usage

Show helps:

```bash
turn --help
```

#### Command-line arguments
> Command-line arguments take precedence over environment variables

| values          | default        | env            | tips                       |
|-----------------|----------------|----------------|----------------------------|
| --realm         | localhost      | TURN_REALM     | turn working relam         |
| --external      | 127.0.0.1:3478 | TURN_EXTERNAL  | turn server public address |
| --listening     | 127.0.0.1:3478 | TURN_LISTENING | turn server udp bind port  |
| --nats          | 127.0.0.1:4222 | TURN_NATS      | nats server connection url |
| --threads       |                | TURN_THREADS   | internal thread pool size  |

for sys calls, multithreading does not significantly help to improve IO throughput.

#### Simple example

Set envs:

```bash
export TURN_EXTERNAL="127.0.0.1:3478"
export TURN_LISTEN="127.0.0.1:3478"
```

Or else use command-line arguments:

```bash
turn --listening=127.0.0.1:8080 --external=127.0.0.1:8080
```

#### Logs

The server closes log output by default, and the log output level can be set using environment variables:

```bash
export RUST_LOG=<level> // error | warn | info | debug | trace
```


## Rpc
> payload type is JSON

#### Public response

```text
* error `{Option<String>}` - error info.
* data `{Option<T>}` - response data.
```

#### Auth - `turn.auth`

Request:

```text
* addr `{SocketAddr}` - udp client session address.
* realm `{String}` - turn server realm.
* username `{String}` - session username.
```

Response:

```text
* password `{String}` - session password.
* group `{u32}` - session group id.
```

#### Close - `turn.<realm>.close`

Request:

```text
* addr `{String}` - session address.
```


## Building

```bash
cargo build --release
```

Compile the program and link library from the source code, turn on O3 optimization, turn off overflow checking and debug information, and exit directly when a panic occurs.
The compiled output product is located in the `target/release` directory under the current project, and the output product will be different for different platforms.


## Docker

```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
COPY ./.cargofile ./.cargo/config

RUN cargo build --release
RUN cp -r ./target/release/turn /usr/local/bin/turn \
    && chmod +x /usr/local/bin/turn

FROM ubuntu:latest
COPY --from=builder /usr/local/bin/turn /usr/local/bin/turn
EXPOSE 3478/udp
CMD turn
```


## Author

[Mr.Panda](https://github.com/mycrl) - xivistudios@gmail.com
