<!--lint disable no-literal-urls-->
<div align="center">
  <h1>TURN-RS</h1>
</div>
<br/>
<div align="center">
  <strong>一个纯 ❤️ Rust实现的TURN服务器</strong>
</div>
<div align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/mycrl/turn-rs/cargo-test.yml?branch=main"/>
  <img src="https://img.shields.io/github/license/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/issues/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/stars/mycrl/turn-rs"/>
</div>
<br/>
<br/>

一个完全由Rust实现的STUN/TURN服务器, 跟现在广泛引用的coturn项目不同的是, 这个项目提供了更灵活的外部接口并提供与coturn相同的性能和资源开销（也许更好）, 非常适合在WebRTC项目中需要转发的场景下使用.


## 谁在使用它?

* [`Psyai`](https://psyai.com)
* [`Faszialespecialist`](https://faszialespecialist.com/)


## 目录

* [特性](#特性)
* [使用](#使用)
  * [docker](#docker)  
  * [linux service](#linux-service)
* [编译](#编译)


## 特性

- 支持UDP和TCP传输层.
- 外部钩子接口. <sup>([`hooks-api`])</sup>
- 外部控制接口. <sup>([`controller-api`])</sup>
- 配置文件中的静态身份认证.
- 只允许长期身份认证.
- 虚拟端口分配. <sup>(`并不会真正分配真实的端口`)</sup>

[`controller-api`]: https://github.com/mycrl/turn-rs/wiki/Controller-API-Reference
[`hooks-api`]: https://github.com/mycrl/turn-rs/wiki/Hooks-API-Reference


## 使用

> docker和crates.io上的版本可能非常落后, 推荐从源代码编译.

```bash
cargo install turn-server
```

从配置文件启动:

```bash
turn-server --config=/etc/turn_server/config.toml
```

详细配置项请查看示例配置文件: [turn_server.toml](./turn_server.toml)


#### Docker

```bash
docker pull quasipaa/turn-server
```
自定义配置文件`/etc/turn-server/config.toml`通过`-v`覆盖镜像内的路径.

#### Linux service

```
./install-service.sh
```

这将编译项目并安装并启动服务.


## 编译

#### 依赖项

你需要先安装[Rust](https://www.rust-lang.org/tools/install), 如果已经安装了可以跳过, 然后获取源代码:

```bash
git clone https://github.com/mycrl/turn-rs
```

#### 编译工作区

在发布模式下编译整个工作区:

```bash
cd turn-rs
cargo build --release
```

编译完成后, 可以在`"target/release"`目录下找到二进制文件.


## License

[MIT](./LICENSE)
Copyright (c) 2022 Mr.Panda.
