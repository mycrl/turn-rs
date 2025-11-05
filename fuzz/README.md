# Fuzz Testing for turn-server

This directory contains fuzz tests for the TURN server implementation using `cargo-fuzz` and libFuzzer.

## Fuzz Targets

### 1. `fuzz_stun_decoder`
Tests the core STUN/TURN message decoder against malformed and random inputs. This is the primary fuzz target for finding crashes and panics in the codec.

### 2. `fuzz_stun_message`
Focuses on STUN message parsing and encoding round-trip stability. Tests that successfully decoded messages can be properly accessed and re-encoded.

### 3. `fuzz_channel_data`
Tests TURN ChannelData parsing and encoding. ChannelData is used for efficient data forwarding in TURN.

### 4. `fuzz_stun_attributes`
Tests parsing of all STUN/TURN attribute types including addresses, strings, numbers, and complex structures.

## Prerequisites

Install cargo-fuzz:
```bash
cargo install cargo-fuzz
```

## Running Fuzz Tests

Run a specific fuzz target:
```bash
# From the repository root
cargo fuzz run fuzz_stun_decoder

# Run with limited time (e.g., 60 seconds)
cargo fuzz run fuzz_stun_decoder -- -max_total_time=60

# Run with specific number of runs
cargo fuzz run fuzz_stun_message -- -runs=100000
```

Run all fuzz targets sequentially:
```bash
cargo fuzz run fuzz_stun_decoder -- -max_total_time=30
cargo fuzz run fuzz_stun_message -- -max_total_time=30
cargo fuzz run fuzz_channel_data -- -max_total_time=30
cargo fuzz run fuzz_stun_attributes -- -max_total_time=30
```

## Analyzing Results

When a crash is found, it will be saved to `fuzz/artifacts/<target_name>/`:
```bash
# List crashes
ls fuzz/artifacts/fuzz_stun_decoder/

# Reproduce a crash
cargo fuzz run fuzz_stun_decoder fuzz/artifacts/fuzz_stun_decoder/crash-<hash>
```

## Coverage

Generate coverage report:
```bash
cargo fuzz coverage fuzz_stun_decoder
```

## Continuous Fuzzing

For continuous integration, you can run fuzz tests with a time limit:
```bash
# Run each target for 5 minutes
for target in fuzz_stun_decoder fuzz_stun_message fuzz_channel_data fuzz_stun_attributes; do
    cargo fuzz run $target -- -max_total_time=300 || true
done
```

## Corpus

The corpus directory contains interesting inputs discovered during fuzzing. You can seed it with sample STUN/TURN messages from `tests/samples/` for better coverage:

```bash
# Copy sample messages to corpus
mkdir -p fuzz/corpus/fuzz_stun_decoder
cp tests/samples/*.bin fuzz/corpus/fuzz_stun_decoder/ 2>/dev/null || true
```

## Best Practices

1. **Start with short runs**: Use `-max_total_time=60` initially to verify targets work
2. **Use sample data**: Seed corpus with valid messages from tests/samples/
3. **Monitor memory**: Use `-rss_limit_mb=2048` to limit memory usage
4. **Parallel fuzzing**: Run different targets in parallel for better coverage
5. **Regular runs**: Integrate into CI with time limits (e.g., 5 minutes per target)

## Troubleshooting

If fuzzing fails to start:
- Ensure you're using nightly Rust: `rustup default nightly`
- Rebuild fuzz targets: `cargo fuzz build`
- Check for compilation errors: `cargo build --manifest-path fuzz/Cargo.toml`

## References

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer options](https://llvm.org/docs/LibFuzzer.html#options)
- RFC 5389 (STUN): https://tools.ietf.org/html/rfc5389
- RFC 5766 (TURN): https://tools.ietf.org/html/rfc5766
