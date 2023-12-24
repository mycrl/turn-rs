FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN sudo apt update && \
    sudo apt install -y protobuf-compiler libprotobuf-dev && \
    cargo build --release

FROM debian:buster-slim
WORKDIR /app
RUN apt update && \
    apt-get install pkg-config libssl-dev -y
COPY --from=builder /app/target/release/turn-server /usr/local/bin/turn-server
COPY --from=builder /app/turn_server.toml /etc/turn-server/config.toml
CMD turn-server --config=/etc/turn-server/config.toml
