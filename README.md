<h1 align="center">
    <img src="./material/logo.svg" width="300px"/>
</h1>
<div align="center">
    <strong>Rust ❤️ WebRTC STUN/TURN Server</strong>
</div>
<div align="center">
    <img src="https://img.shields.io/github/workflow/status/Mycrl/Mysticeti/Mysticeti Tests"/>
    <img src="https://img.shields.io/github/languages/top/Mycrl/Mysticeti"/>
    <img src="https://img.shields.io/github/license/Mycrl/Mysticeti"/>
    <img src="https://img.shields.io/badge/author-Mr.Panda-read"/>
</div>
<br/>
<br/>

The project does not intend to support the complete RFC specification. In the short term, it only supports the UDP protocol, and some changes and simplifications have been made to some specifications to facilitate implementation, and other functions may be added in the future. This is an experimental project, the more purpose is to experiment and explore.


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

Because the project uses script to generate unit tests, you cannot directly use "cargo test" to run tests. you need to perform automated tests through internal script:
```bash
cd ./tests
npm run unit-tests
```

### License
[MIT](./LICENSE)
Copyright (c) 2020 Mr.Panda.
