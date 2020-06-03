FROM ubuntu:18.04
MAINTAINER Mr.Panda xivistudios@gmail.com

ENV TimeZone=Asia/Shanghai   
RUN ln -snf /usr/share/zoneinfo/$TimeZone /etc/localtime \
    && echo $TimeZone > /etc/timezone

COPY ../target/release/exchange /etc/exchange
COPY ../default_config.toml /etc/exchange_config.toml

RUN chmod -R 755 /etc/exchange

WORKDIR /etc
EXPOSE 1936
CMD ["./exchange -c ./exchange_config.toml"]