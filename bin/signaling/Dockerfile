FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release
RUN cp -r ./target/release/signaling /usr/local/bin/signaling \
    && chmod +x /usr/local/bin/signaling

FROM ubuntu:latest
COPY --from=builder /usr/local/bin/signaling /usr/local/bin/signaling
EXPOSE 3478/udp
CMD signaling
