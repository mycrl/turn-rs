FROM ubuntu:18.04
MAINTAINER Mr.Panda xivistudios@gmail.com

ENV TimeZone=Asia/Shanghai   
RUN ln -snf /usr/share/zoneinfo/$TimeZone /etc/localtime \
    && echo $TimeZone > /etc/timezone

COPY ../target/release/publish /etc/publish
COPY ../default_config.toml /etc/publish_config.toml

RUN chmod -R 755 /etc/publish

WORKDIR /etc
EXPOSE 1936
CMD ["./publish -c ./publish_config.toml"]