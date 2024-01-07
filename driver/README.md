<!--lint disable no-literal-urls-->
<div align="center">
  <h1>turn-driver</h1>
</div>
<br/>
<div align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/mycrl/turn-rs/tests.yml?branch=main"/>
  <img src="https://img.shields.io/github/license/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/issues/mycrl/turn-rs"/>
  <img src="https://img.shields.io/github/stars/mycrl/turn-rs"/>
</div>
<div align="center">
  <sup>A turn server driver for rust.</sup>
</div>
</br>
</br>

This is a turn-rs driver library provided for use by rust. The driver can implement turn external control, turn external hooks, and balance delay detection functions.


## Examples

* [balance](./examples/balance.rs) - Send a latency detection request to the turn balance server at address `127.0.0.1:3001`.  
* [controller](./examples/controller.rs) - A simple external controller that only gets status information from the turn server.
* [hooks](./examples/hooks.rs) - A simple external hook service that handles turn server requests to get passwords.


## Start Examples

A quick example can be run through `cargo`:

```rust
cargo run --example [example name]
```

Replace `example name` with one from the list of examples below.


## License

[GPL](../LICENSE) Copyright (c) 2022 Mr.Panda.
