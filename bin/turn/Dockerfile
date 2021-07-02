FROM rust:latest as builder
WORKDIR app
COPY . .
RUN cd ./bin/turn \
    && cargo build --release
RUN cp -r ./target/release/turn /usr/local/bin/turn \
    && chmod +x /usr/local/bin/turn

FROM ubuntu:latest
ENV RUST_LOG=info
COPY --from=builder /usr/local/bin/turn /usr/local/bin/turn
COPY --from=builder /app/bin/turn/entrypoint.sh /usr/local/src/entrypoint.sh
RUN chmod +x /usr/local/src/entrypoint.sh
EXPOSE 3478/udp
CMD /usr/local/src/entrypoint.sh