# WebRTC Demo

This interactive demo creates two local `RTCPeerConnection`s that are forced to relay
through your turn-rs server (`iceTransportPolicy: "relay"`), captures your screen, and
streams it from the "local" peer to the "remote" peer entirely via TURN.

## How to use

1. Start a turn-rs server reachable from your browser (default signaling assumes
   `turn:<server>:3478`, credential `test`).
2. Fill in the **server** address and pick the **transport** (`udp`/`tcp`) and **auth**
   user in both panels.
3. Click the **local** video to start screen capture and begin relaying.

> Screen capture requires a secure context (`https://` or `localhost`) and your
> browser's permission. The demo loads Vue from a CDN, so an internet connection is
> needed the first time.

<iframe
  src="./webrtc-demo.html"
  title="turn-rs WebRTC Demo"
  style="width: 100%; height: 600px; border: 1px solid var(--vp-c-divider); border-radius: 8px; margin-top: 16px;"
  allow="display-capture; camera; microphone"
></iframe>

<p style="margin-top: 12px;">
  <a href="./webrtc-demo.html" target="_blank" rel="noreferrer">Open the demo in a new tab ↗</a>
</p>
