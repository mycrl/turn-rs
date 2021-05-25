FROM ubuntu:latest
MAINTAINER Mr.Panda xivistudios@gmail.com

ENV TimeZone=Asia/Shanghai
ENV PATH="/root/.cargo/bin:${PATH}"
ENV RUST_LOG=info

RUN ln -snf /usr/share/zoneinfo/$TimeZone /etc/localtime \
    && echo $TimeZone > /etc/timezone
    
RUN apt update \
    && apt install -y curl software-properties-common build-essential libssl-dev pkg-config \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && mkdir /usr/local/src/Mysticeti

COPY ./src /usr/local/src/turn/src
COPY ./Cargo.toml /usr/local/src/turn/Cargo.toml
COPY $MYSTICETI_CONFIG $MYSTICETI_CONFIG

RUN cd /usr/local/src/turn \
    && cargo build --release \
    && cp /usr/local/src/turn/target/release/turn /usr/local/bin/turn \
    && chmod +x /usr/local/bin/turn
    
EXPOSE 3478
CMD turn --listen 0.0.0.0:3478
