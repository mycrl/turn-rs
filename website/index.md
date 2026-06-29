---
layout: home

hero:
    name: turn-rs
    text: TURN Server implemented in Rust
    tagline: A high-performance, lightweight TURN/STUN server for WebRTC NAT traversal and media relay — near out-of-the-box, minimal configuration.
    actions:
        - theme: brand
          text: Get Started
          link: /guide/install
        - theme: alt
          text: View on GitHub
          link: https://github.com/mycrl/turn-rs

features:
    - icon: ⚡
      title: Blazing Fast
      details: Processes tens of millions of channel-data forwarding messages per second within a single thread, with forwarding latency below 35 microseconds.
    - icon: 🪶
      title: Lightweight & Simple
      details: Prioritizes core functionality with minimal configuration cost. Runs great everywhere — even on a Raspberry Pi — and stays fast under heavy client load.
    - icon: 🔌
      title: Extensible & Observable
      details: Optional gRPC management API, dynamic Hook-based auth and lifecycle events, plus a built-in Prometheus metrics exporter.
---
