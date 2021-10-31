FROM rust:latest as builder
WORKDIR app
COPY . .
RUN cd ./bin/turn \
    && cargo build --release
RUN cp -r ./target/release/turn /usr/local/bin/turn \
    && chmod +x /usr/local/bin/turn

FROM ubuntu:latest
COPY --from=builder /usr/local/bin/turn /usr/local/bin/turn
EXPOSE 3478/udp
CMD turn