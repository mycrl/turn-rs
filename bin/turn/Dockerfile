FROM rust:latest as builder

WORKDIR /usr/local/src
COPY . .

RUN cd ./bin/turn \
    && cargo build --release
RUN cp -r ./target/release/turn /usr/local/bin/turn \
    && chmod +x /usr/local/bin/turn


FROM ubuntu:latest

WORKDIR /usr/local/src
COPY --from=builder /usr/local/bin/turn /usr/local/bin/turn
COPY --from=builder /usr/local/src/bin/turn/entrypoint.sh /usr/local/src/entrypoint.sh

RUN chmod +x ./entrypoint.sh

EXPOSE 3478
ENV RUST_LOG=info

CMD ./entrypoint.sh