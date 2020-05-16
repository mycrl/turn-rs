# Quasipaa

Build a large-scale high-availability and high-performance streaming media cluster mainly built with Rust programming language,
This is a new attempt and a good start, Hope to rely on Rust's high performance and elegant design to go further and further in the field of streaming media.


### Design
> Including push stream processing, media protocol processing, live on-demand services, and including providing highly available load balancing services for multi-service clusters.

```rust
                                                        +------+
                                                        | core |
                                                        +------+
                                                            |
                                           /------------------------------\
                               
                                        +---------+    +----------+    +------+
                               push ->  | publish | -> | exchange | -> | pull | -> player
                                        +---------+    +----------+    +------+
                                                            |
                                                        +--------+           
                                                        | static |  
                                                        +--------+
```


### Version
Development and design phase </br>


### Plan
> Early plans only supported RTMP, HLS, HttpFLV protocols, And currently only consider H264 encoding, 
Support for WebRTC and VP9 in future plans.</br>

* [x] Rtmp push stream processing</br>
* [ ] Data Exchange Center</br>
* [ ] Load balancing service</br>
* [ ] Audio and video processing</br>
* [ ] Live service</br>
* [ ] replay and static file support</br>


### Overview
Quasipaa is a streaming media service group written in Rust programming language, including multiple independent services that can be scaled horizontally:
* Stream push service for handling live stream push.</br>
* Media Data Exchange Center handles mixed sources.</br>
* Control center that provides load balancing and cluster management for multiple horizontal services.</br>
* Static file and replay recording service.</br>
* Multi-protocol live processing service.</br>


### Future
* Push Stream SDK.</br>
* Self-developed streaming protocol.</br>
* Support as many existing protocols as possible.</br>
* Perfect performance.</br>
* WebRTC's TURN support.</br>
* Adaptive multiple coding.</br>


### License
[GPL](./LICENSE)
Copyright (c) 2020 Mr.Panda.
