<!--lint disable no-literal-urls-->
<div align="center">
  <img src="../logo.svg" width="200px"/>
</div>
<br/>
<div align="center">
  <strong>TURN Server implemented by ❤️ Rust</strong>
</div>
<div align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/mycrl/turn-rs/tests.yml?branch=main"/>
  <img src="https://img.shields.io/github/license/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/issues/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/stars/mycrl/turn-rs"/>
</div>
<div align="center">
  <sup>RFC: https://datatracker.ietf.org/doc/html/rfc8656</sup>
</div>
<br/>
<br/>

A pure Rust implementation of the turn server. Compared with coturn, the advantage is to provide better performance. Single-threaded decoding speed up to 5Gib/s, forwarding delay is less than 35 microseconds. However, it does not provide as rich as coturn feature support, this project is more focused on the core business, do not need to touch the complex configuration items, almost out of the box.


## Usage

> The version on crates.io can be very outdated. It is recommended to compile directly from the github source or download the compiled binary from the [release](https://github.com/mycrl/turn-rs/releases).

Start with configuration file:

```bash
turn-server --config=/etc/turn-server/config.toml
```

Please check the example configuration file for details: [turn-server.toml](../turn-server.toml)


#### Docker

```bash
docker pull ghcr.io/mycrl/turn-server
```
The custom configuration file overrides the `/etc/turn-server/config.toml` path inside the image through `-v`.

#### Linux service

```
./install-service.sh
```

This will compile the project and install and start the service.


## License

[GPL](../LICENSE) Copyright (c) 2022 Mr.Panda.
