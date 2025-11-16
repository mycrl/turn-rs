# Build

### Prerequisites

You need to install the Rust toolchain, if you have already installed it, you can skip it, Install Rust, then get the source code:

```bash
git clone https://github.com/mycrl/turn-rs
```

### Build Workspace

Compile the entire workspace in release mode:

```bash
cd turn-rs
cargo build --release
```

You can enable target CPU optimizations, which will enable optimizations based on your current CPU. This can be easily enabled by adding an environment variable before compiling:

```bash
export RUSTFLAGS='-C target-cpu=native'
```

### Features

If you don't need a particular feature, you can reduce the package size by enabling only the features you require.

-   `udp` - (enabled by default) Enables UDP transport layer support.
-   `tcp` - Enables TCP transport layer support.
-   `ssl` - Enable SSL encryption support.
-   `grpc` - Enable the GRPC server feature.

All features are enabled by default.

```bash
cargo build --release
```

After the compilation is complete, you can find the binary file in the `target/release` directory.
