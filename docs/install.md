# Install

turn-rs is cross-platform, supports Linux, Windows, macOS, and provides a Docker image.

First, download the corresponding binary file for your platform from [github release](https://github.com/mycrl/turn-rs/releases). If your current platform and operating system are not provided in the release, you can jump to [build](./build.md) to compile it yourself. Don't panic, because compiling turn-rs is very simple.

### Docker

The docker image is published to [github packages](https://github.com/mycrl/turn-rs/pkgs/container/turn-server), and no docker.io image is provided. Using this image is very simple:

```bash
docker pull ghcr.io/mycrl/turn-server:latest
```

It should be noted that using this image requires a custom configuration file. You can use the `-v` option to override the default configuration file path inside the image. The default configuration file path is `/etc/turn-server/config.toml`
