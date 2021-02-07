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

COPY ./src /usr/local/src/Mysticeti/src
COPY ./Cargo.toml /usr/local/src/Mysticeti/Cargo.toml
COPY $MYSTICETI_CONFIG $MYSTICETI_CONFIG

RUN cd /usr/local/src/Mysticeti \
    && cargo build --release \
    && cp /usr/local/src/Mysticeti/target/release/mysticeti /usr/local/bin/Mysticeti \
    && chmod +x /usr/local/bin/Mysticeti
    
EXPOSE 3478
CMD ["Mysticeti"]