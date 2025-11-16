FROM rust:bookworm AS builder
RUN apt-get update && \
    apt-get install -y protobuf-compiler && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/
COPY . .
RUN cargo build --release --all-features
    
FROM debian:bookworm-slim
WORKDIR /app
RUN apt update && \
    apt-get install pkg-config libssl-dev -y && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/target/release/turn-server /usr/local/bin/turn-server
COPY --from=builder /usr/src/turn-server.json /etc/turn-server/config.json
CMD turn-server --config=/etc/turn-server/config.json
