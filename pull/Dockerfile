FROM ubuntu:18.04
MAINTAINER Mr.Panda xivistudios@gmail.com

ENV TimeZone=Asia/Shanghai   
RUN ln -snf /usr/share/zoneinfo/$TimeZone /etc/localtime \
    && echo $TimeZone > /etc/timezone

COPY ../target/release/pull /etc/pull
COPY ../default_config.toml /etc/pull_config.toml

RUN chmod -R 755 /etc/pull

WORKDIR /etc
EXPOSE 1936
CMD ["./pull -c ./pull_config.toml"]