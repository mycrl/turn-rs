# Quasipaa

![ci](https://img.shields.io/github/workflow/status/quasipaas/quasipaa/CI)
![languages](https://img.shields.io/github/languages/top/quasipaas/quasipaa)
![open issues](https://img.shields.io/github/issues/quasipaas/quasipaa)
![pull requests](https://img.shields.io/github/issues-pr/quasipaas/quasipaa)
![license](https://img.shields.io/github/license/quasipaas/quasipaa)
![forks](https://img.shields.io/github/forks/quasipaas/quasipaa)
![stars](https://img.shields.io/github/stars/quasipaas/quasipaa)
![release](https://img.shields.io/github/v/release/quasipaas/quasipaa)
![last commit](https://img.shields.io/github/last-commit/quasipaas/quasipaa)
![author](https://img.shields.io/badge/author-Mr.Panda-read)

Quasipaa is an open source, distributed real-time audio and video server. Unlike other solutions, this is not a library that can be embedded in your own program, but a full-fledged cluster of services, Quasipaa not only provides basic streaming media server, but also includes load balancing, external control API interface. Quasipaa by Rust to build, the use of Rust outstanding performance and excellent engineering became a robust system.

Quasipaa（[Quasipaa spinosa](https://en.wikipedia.org/wiki/Quasipaa_spinosa) is a species of frog in the family Dicroglossidae）The project was originally created to address the Rust's lack of audio and video servers, and when the author started the project, the area was virtually empty.


### Version
Development stage</br>
The progress of the project is in the [project dashboard](https://github.com/quasipaas/Quasipaa/projects/1), you can check it at any timeoi.

> **Note:**
>
> Due to the limited ability of the author, the early plan only supports RTMP, WebRTC and HttpFLV protocols. I will try to improve the support for different protocols and codes in the later stage.


### Overview
![design](./design.svg)

Quasipaa is a streaming media service cluster, which contains multiple independent services, which can be scaled horizontally:
- **Publish:** 
    The data pushed through various protocols will be pushed to the exchange after processing and demultiplexing.
- **Exchange:** 
    The exchange further processes the Publish data, such as secondary compression, saving as static files, encoding conversion......
- **Core:** 
    Controls all nodes of the cluster, including load balancing, dynamic scheduling,  authority control, coding and protocol control.
* **Object Storage:** 
    Storing log data and live replay as static files......
* **Pull:** 
    The data is repackaged into multiple protocols and distributed to clients.


### Deployment
It is currently in development and has not completed all the features of stage 1, so actual deployment is not supported.


### Plan
* [x] RTMP protocol support.
* [x] Exchange.
* [x] Load balancing.
* [ ] Audio video data codec.
* [x] Live service.
* [ ] Live playback and static file support.
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
