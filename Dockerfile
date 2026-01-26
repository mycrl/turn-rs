FROM rust:bookworm AS builder
RUN apt-get update && \
    apt-get install -y protobuf-compiler && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/
COPY . .
RUN cargo build --release --all-features
    
FROM debian:bookworm-slim
COPY --from=builder /usr/src/target/release/turn-server /usr/local/bin/turn-server
COPY --from=builder /usr/src/turn-server.toml /etc/turn-server/config.toml
CMD turn-server --config=/etc/turn-server/config.toml
