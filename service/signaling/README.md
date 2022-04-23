<!--lint disable no-literal-urls-->
<br/>
<br/>
<div align="center">
  <img 
    alt="mystery"
    src="../../logo.svg" 
    width="200px"
  />
</div>
<br/>
<div align="center">
  <strong>Mystery - Signaling</strong>
</div>
<br/>


WebRTC signaling implementation, the project is provided in the form of a library and a separate executable program, and the server is implemented to use WebSocket as the transport protocol, providing external authentication support and fearless concurrency.


## Docs

```bash
cargo doc --open
```


## Usage

Show helps:

```bash
signaling --help
```

#### Command-line arguments
> Command-line arguments take precedence over environment variables

| values                        | default        | env                          | tips                                                                  |
|-------------------------------|----------------|------------------------------|-----------------------------------------------------------------------|
| --realm                       | localhost      | SIGAALING_REALM              | signaling working relam                                               |
| --listening                   | 127.0.0.1:80   | SIGAALING_LISTENING          | signaling server udp bind port                                        |
| --nats                        | 127.0.0.1:4222 | SIGAALING_NATS               | nats server connection url                                            |
| --max_send_queue              |                | SIGAALING_MAX_SEND_QUEUE     | the size of the send queue                                            |
| --max_message_size            |                | SIGAALING_MAX_MESSAGE_SIZE   | the maximum size of a message                                         |
| --max_frame_size              |                | SIGAALING_MAX_FRAME_SIZE     | the maximum size of a single message frame                            |
| --accept_unmasked_frames      | false          | SIGAALING_UNMASKED_FRAMES    | the server will accept and handle unmasked frames from the client     |

#### Simple example

Set envs:

```bash
export SIGAALING_NATS="127.0.0.1:4222"
export SIGAALING_LISTENING="127.0.0.1:8080"
```

Or else use command-line arguments:

```bash
signaling --listening=127.0.0.1:8080 --nats=127.0.0.1:4222
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

#### Auth - `signaling.auth`

Request:

```text
* realm `{String}` - websocket session address.
* uid `{String}` - user id.
* token `{String}` - request token.
```


## Interface

```text
protocol://hostname:port/:uid?:token
```

Example url:
```text
ws://localhost/QOpJurAGyto6?8lEtnf8IODB1EsAFf8eU
               |-- uid ---| |------ token -----|
```


## Message struct

### SignalingMessage

| fields                        | type                                            | optional   | tips                           |
|-------------------------------|-------------------------------------------------|------------|--------------------------------|
| to                            | string                                          | false      | target uid                     |
| from                          | string                                          | false      | source uid                     |
| type                          | Types                                           | false      | signaling message types        |
| data                          | SignalingMessageDesc / SignalingMessageCandiate | true       | signaling message payload      |


### Types

| values                        | for data field type                             |
|-------------------------------|-------------------------------------------------|
| "offer"                       | SignalingMessageDesc                            |
| "answer"                      | SignalingMessageDesc                            |
| "candidate"                   | SignalingMessageCandiate                        |
| "connect"                     | empty                                           |
| "disconnect"                  | empty                                           |


### SignalingMessageDesc

| fields                        | type                                            | optional   | tips                           |
|-------------------------------|-------------------------------------------------|------------|--------------------------------|
| sdp                           | string                                          | false      | offer or answer sdp info       |


### SignalingMessageCandiate

| fields                        | type                                            | optional   | tips                           |
|-------------------------------|-------------------------------------------------|------------|--------------------------------|
| candidate                     | string                                          | false      | candidate info                 |
| sdpMid                        | string                                          | false      |                                |
| sdpMLineIndex                 | int                                             | false      |                                |


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
COPY ./ ./
COPY ./.cargofile ./.cargo/config

RUN cargo build --release
RUN cp -r ./target/release/signaling /usr/local/bin/signaling \
    && chmod +x /usr/local/bin/signaling

FROM ubuntu:latest
COPY --from=builder /usr/local/bin/signaling /usr/local/bin/signaling
EXPOSE 80/tcp
CMD signaling
```


## License

[GPL](./LICENSE)
Copyright (c) 2020 Mr.Panda.
