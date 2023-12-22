<!--lint disable no-literal-urls-->
<div align="center">
  <h1>TURN-RS</h1>
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

纯rust实现的turn服务器, 与coturn相比, 优点是提供更好的性能. 单线程解码速度可达5Gib/s, 转发时延小于35微秒. 但是它并没有提供像coturn一样丰富的功能支持, 这个项目最适合webrtc中使用stun/turn服务器的场景. 


## 谁在使用它?

* [`Psyai`](https://psyai.com) <sup>(turn-rs已使用一年多, 没有任何故障或停机.)</sup>
* [`Faszialespecialist`](https://faszialespecialist.com/)


## 目录

* [特性](#特性)
* [使用](#使用)
  * [docker](#docker)  
  * [linux service](#linux-service)
* [编译](#编译)


## 特性

- 传输层支持tcp和udp协议, 支持绑定多个网卡或接口. 
- 可以使用WebHooks api, turn服务器可以主动通知外部服务一些事件并使用外部认证机制.  <sup>([`hooks-api`])</sup>
- 外部控制API, 外部各方可以主动控制turn服务器并管理会话. <sup>([`controller-api`])</sup>
- 静态认证列表可以在配置文件中使用. 
- 仅支持长期身份验证机制. 
- 始终只分配虚拟端口, 不占用实际系统端口. 

[`controller-api`]: https://github.com/mycrl/turn-rs/wiki/Controller-API-Reference
[`hooks-api`]: https://github.com/mycrl/turn-rs/wiki/Hooks-API-Reference


## 使用

> docker和crates.io上的版本可能非常落后,  推荐从源代码编译.

```bash
cargo install turn-server
```

从配置文件启动:

```bash
turn-server --config=/etc/turn_server/config.toml
```

详细配置项请查看示例配置文件: [turn_server.toml](./turn_server.toml)
请参阅[wiki](https://github.com/mycrl/turn-rs/wiki/Configuration)以获取配置文件的说明. 


#### Docker

```bash
// docker hub
docker pull quasipaa/turn-server
// github packages
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

你需要先安装[Rust](https://www.rust-lang.org/tools/install),  如果已经安装了可以跳过,  然后获取源代码:

```bash
git clone https://github.com/mycrl/turn-rs
```

#### 编译工作区

在发布模式下编译整个工作区:

```bash
cd turn-rs
cargo build --release
```

编译完成后,  可以在`"target/release"`目录下找到二进制文件.


## License

[GPL](./LICENSE)
Copyright (c) 2022 Mr.Panda.
