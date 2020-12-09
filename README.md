# Quasipaa

![languages](https://img.shields.io/github/languages/top/quasipaa/Spinosa)
![open issues](https://img.shields.io/github/issues/quasipaa/Spinosa)
![pull requests](https://img.shields.io/github/issues-pr/quasipaa/Spinosa)
![license](https://img.shields.io/github/license/quasipaa/Spinosa)
![forks](https://img.shields.io/github/forks/quasipaa/Spinosa)
![stars](https://img.shields.io/github/stars/quasipaa/Spinosa)
![last commit](https://img.shields.io/github/last-commit/quasipaa/Spinosa)
![author](https://img.shields.io/badge/author-Mr.Panda-read)

##### My current work is focused on the WebRTC server. I temporarily put this project on hold and will re-plan the project after completing the above work.

Quasipaa is an open source, distributed real-time audio and video server. Unlike other solutions, this is not a library that can be embedded in your own program, but a full-fledged cluster of services, Quasipaa not only provides basic streaming media server, but also includes load balancing, external control API interface. Quasipaa by Rust to build, the use of Rust outstanding performance and excellent engineering became a robust system.

Quasipaa（[Quasipaa spinosa](https://en.wikipedia.org/wiki/Quasipaa_spinosa) is a species of frog in the family Dicroglossidae）The project was originally created to address the Rust's lack of audio and video servers, and when the author started the project, the area was virtually empty.


### Version
Development stage</br>
The progress of the project is in the [project dashboard](https://github.com/quasipaas/Quasipaa/projects/1), you can view it at any time.

> **Note:**
> Due to the limited ability of the author, the early plan only supports RTMP, WebRTC and HttpFLV protocols. I will try to improve the support for different protocols and codec in the later stage.


### Deployment
It is currently in development and has not completed all the features of stage 1, so actual deployment is not supported.


### Overview
![design](./design.svg)

Quasipaa is a streaming media service cluster, which contains multiple independent services, which can be scaled horizontally:
* `Publish:` The data pushed through various protocols will be pushed to the exchange after processing and demultiplexing.
* `Exchange:` The exchange further processes the Publish data, such as secondary compression, saving as static files, encoding conversion.
* `Core:` Controls all nodes of the cluster, including load balancing, dynamic scheduling,  authority control, coding and protocol control.
* `Object Storage:` Storing log data and live replay as static files.
* `Pull:` The data is repackaged into multiple protocols and distributed to clients.


### Plan
* [x] RTMP protocol support.
* [x] Exchange.
* [x] Load balancing.
* [ ] Audio video data codec.
* [x] Live service.
* [ ] Live playback and static file support.
* [x] WebRTC STUN support.
* [ ] WebRTC TURN support.


### Roadmap
* WebAssembly SDK.
* Independently developed live protocols.
* Support existing protocols wherever possible.
* Best performance possible.
* Adaptive multiple codec.


### License
[GPL](./LICENSE)
Copyright (c) 2020 Mr.Panda.
