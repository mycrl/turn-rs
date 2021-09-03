<!--lint disable no-literal-urls-->
<div align="center">
  <img 
    alt="mystery"
    src="./logo.svg" 
    width="200px"
  />
</div>
<br/>
<div align="center">
  <strong>WebRTC Server implemented by ❤️ Rust</strong>
</div>
<div align="center">
  <img src="https://img.shields.io/github/workflow/status/Mycrl/mystery/cargo-test"/>
  <img src="https://img.shields.io/github/license/Mycrl/mystery"/>
  <img src="https://img.shields.io/github/issues/Mycrl/mystery"/>
  <img src="https://img.shields.io/github/stars/Mycrl/mystery"/>
</div>
<br/>
<br/>

mystery is a WebRTC server solution implemented using Rust and supports the SFU/MCU model. Compared with other ongoing projects, the current project prioritizes WebRTC one-to-many live broadcasting, but this does not mean that the project will give up peer-to-peer two-way dialogue.

## Table of contents

* [Roadmap](#roadmap)
* [Building](#building)
  * [Prerequisites](#prerequisites)
  * [Build workspace](#build-workspace)
  * [Turn Server](#turn-server)
* [Code style](#code-style)

## Roadmap

- [x] TURN
  - [x] STUN
- [ ] WebRTC
  - [x] RTP
  - [ ] RTCP
  - [ ] SRTP
  - [ ] DTLS
  - [x] SDP
  - [ ] ICE
- [ ] SFU
- [ ] MCU

## Building

### Prerequisites

You need to install the Rust toolchain, if you have already installed it, you can skip it, [Install Rust](https://www.rust-lang.org/tools/install), then get the source code:

```bash
git clone https://github.com/Mycrl/mystery
```

And, you need to install the openssl toolchain.

#### Windows

If you have [chocolatey](https://chocolatey.org/install) installed you can install openssl via a single command i.e.

```bash
choco install openssl
```

#### Linux

```bash
sudo apt-get install libssl-dev
```

#### Macos

```bash
brew install openssl
```

### Build workspace

Compile the entire workspace in release mode:

```bash
cd mystery
cargo build --release
```

After the compilation is complete, you can find the binary file in the "target/release" directory.

### Docker compose

Use docker-compose to start all services:

```bash
cd mystery
docker-compose up -d
```

## Code style

The coding style of this project may not conform to the community style or the habits of most people, but it conforms to my own style. I have paranoid requirements for the code format, I know this is a bad habit, and the current project is also independently developed and maintained by me. If you have more suggestions, you can tell me.

## License

[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FMycrl%2Fmystery.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2FMycrl%2Fmystery?ref=badge_large)

[GPL](./LICENSE)
Copyright (c) 2020 Mr.Panda.
