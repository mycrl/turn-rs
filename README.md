<h1 align="center">
    <img src="./material/logo.svg" width="300px"/>
</h1>
<div align="center">
    <strong>Rust ❤️ WebRTC STUN/TURN Server</strong>
</div>
<div align="center">
    <img src="https://img.shields.io/github/languages/top/kpinosa/Mysticeti"/>
    <img src="https://img.shields.io/github/license/kpinosa/Mysticeti"/>
    <img src="https://img.shields.io/badge/author-Mr.Panda-read"/>
</div>
<br/>
<br/>

The project does not intend to support the complete RFC specification, and some changes and simplifications have been made to some specifications to facilitate implement, Internal TURN is just to ensure full support of WebRTC. 
It should be noted that only the UDP protocol is supported in the short term. Currently, DTLS/TLS (WebRTC's low latency requirements) has not been considered, and other functions may be added in the future, This is an experimental project, There is no plan to meet the production level requirements in the short term, More purpose is to realize and explore.


### Reference

- [RFC 5389](https://tools.ietf.org/html/rfc5389) - Session Traversal Utilities for NAT (STUN)
- [RFC 5766](https://tools.ietf.org/html/rfc5766) - Traversal Using Relays around NAT (TURN)
- [RFC 8489](https://tools.ietf.org/html/rfc8489) - Session Traversal Utilities for NAT (STUN)
- [RFC 8656](https://tools.ietf.org/html/rfc8656) -  Traversal Using Relays around NAT (TURN)


### Installation

first, you need to clone source code from repo:
```bash
git clone https://github.com/Mycrl/Mysticeti.git
```

docker image is the recommended way:
```bash
cd Mysticeti
docker build --tag=mysticeti:latest .
```

runing the image:
```bash
docker run -d --name mysticeti-service mysticeti:latest 
```

you can specify a custom configuration file by passing the `MYSTICETI_CONFIG` environment variable:
```bash
docker run -d -e MYSTICETI_CONFIG=./config.toml --name mysticeti-service mysticeti:latest 
```

or, you can choose to build from source code:
```bash
cd Mysticeti
cargo build --release
cp ./target/release/mysticeti /usr/local/bin/Mysticeti
chmod +x /usr/local/bin/Mysticeti
Mysticeti --help
```

### Testing

Because this project uses automated scripts to generate unit tests, you cannot directly use "cargo test" to run tests. You need to perform automated tests through internal scripts:
```bash
cd ./tests
npm run unit-tests
```

### License
[GPL](./LICENSE)
Copyright (c) 2020 Mr.Panda.
