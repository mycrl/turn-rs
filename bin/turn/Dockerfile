FROM lukemathwalker/cargo-chef as planner
WORKDIR app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM lukemathwalker/cargo-chef as cacher
WORKDIR app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:latest as builder
WORKDIR app
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
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