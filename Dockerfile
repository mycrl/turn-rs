FROM clux/muslrust:stable AS builder
WORKDIR /usr/src/
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl
    
FROM debian:buster-slim
WORKDIR /app
RUN apt update && \
    apt-get install pkg-config libssl-dev -y
COPY --from=builder /app/target/release/turn-server /usr/local/bin/turn-server
COPY --from=builder /app/turn-server.toml /etc/turn-server/config.toml
CMD turn-server --config=/etc/turn-server/config.toml
