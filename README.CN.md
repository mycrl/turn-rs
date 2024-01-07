<!--lint disable no-literal-urls-->
<div align="right">
  <a href="./README.CN.md">简体中文</a>
  /
  <a href="./README.md">English</a>
</div>
<div align="center">
  <img src="./logo.svg" width="200px"/>
</div>
<br/>
<div align="center">
  <strong>一个纯 ❤️ Rust实现的TURN服务器</strong>
</div>
<div align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/mycrl/turn-rs/tests.yml?branch=main"/>
  <img src="https://img.shields.io/github/license/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/issues/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/stars/mycrl/turn-rs"/>
</div>
<br/>
<br/>

Turn服务器的纯Rust实现, 与coturn相比, 优点是提供更好的性能.单线程解码速度高达5Gib/s, 转发延迟小于35微秒.不过它并没有提供像coturn一样丰富的功能支持, 这个项目更专注于核心业务, 不需要接触复杂的配置项, 几乎是开箱即用的.

## 我该如何选择?

#### turn-rs

如果你对coturn配置项不熟悉, 并且对coturn配置项的复杂性感到烦恼, 那么你应该使用这个项目, 同样, 如果你想要更好的性能表现和更低的内存占用, 你也可以使用这个项目.turn-rs配置简单, 外部api也非常简单, 对于核心业务支持已经足够了.

#### coturn

如果您对turn服务器有广泛的标准支持需求, 需要更多的集成服务和生态支持, 那么您应该选择coturn.

## 谁在使用它?

* [`Psyai`](https://psyai.com) <sup>(turn-rs已使用一年多,  没有任何故障或停机.)</sup>
* [`Faszialespecialist`](https://faszialespecialist.com/)


## 目录

* [特性](#特性)
* [使用](#使用)
  * [docker](#docker)  
  * [linux service](#linux-service)
* [编译](#编译)


## 特性

- 仅支持长期身份验证机制.
- 静态认证列表可以在配置文件中使用.
- 始终只分配虚拟端口, 不占用实际系统端口.
- 传输层支持tcp和udp协议, 支持绑定多个网卡或接口.
- 提供简单的命令行工具, 通过命令行工具图形界面来管理和监控turn服务器. <sup>([`turn-cli`])</sup>
- 通过GRPC接口, Turn服务器可以主动向外部服务通知事件并使用外部认证机制, 外部也可以主动控制Turn服务器并管理会话. <sup>([`proto`])</sup>

[`turn-cli`]: ./cli
[`proto`]: ./protos


## 使用

> crates.io 上的版本可能非常过时, 建议直接从 github 源码编译或者从[release](https://github.com/mycrl/turn-rs/releases)下载编译后的二进制文件.

从配置文件启动:

```bash
turn-server --config=/etc/turn_server/config.toml
```

详细配置项请查看示例配置文件: [turn-server.toml](./turn-server.toml)


#### Docker

```bash
docker pull ghcr.io/mycrl/turn-server
```
自定义配置文件`/etc/turn-server/config.toml`通过`-v`覆盖镜像内的路径.

#### Linux service

```
./install-service.sh
```

这将编译项目并安装并启动服务.


## 编译

#### 依赖项

你需要先安装[Rust](https://www.rust-lang.org/tools/install),   如果已经安装了可以跳过,   然后获取源代码:

```bash
git clone https://github.com/mycrl/turn-rs
```

#### 编译工作区

在发布模式下编译整个工作区:

```bash
cd turn-rs
cargo build --release
```

编译完成后,   可以在`"target/release"`目录下找到二进制文件.


## License

[GPL](./LICENSE)
Copyright (c) 2022 Mr.Panda.
