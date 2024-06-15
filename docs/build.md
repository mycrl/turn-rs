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

After the compilation is complete, you can find the binary file in the `target/release` directory.
