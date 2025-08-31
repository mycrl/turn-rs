FROM clux/muslrust:stable AS builder
WORKDIR /usr/src/
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --all-features
    
FROM debian:buster-slim
WORKDIR /app
RUN apt update && \
    apt-get install pkg-config libssl-dev -y
COPY --from=builder /usr/src/target/x86_64-unknown-linux-musl/release/turn-server /usr/local/bin/turn-server
COPY --from=builder /usr/src/turn-server.json /etc/turn-server/config.json
CMD turn-server --config=/etc/turn-server/config.json
