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

这是一个主要使用Rust编程语言构建的实时音视频流服务集群，这是新的尝试和良好的开端.
希望依靠Rust的高性能和优雅的设计在流媒体领域中越走越远.


### 版本
开发阶段 </br>
项目进度更新在 [项目看板](https://github.com/quasipaas/Quasipaa/projects/1)，可以随时跟踪.</br>


### 设计
![design](./design.svg)


### 概述
Quasipaa是使用Rust编程语言编写的流媒体服务集群，其中包括可以水平扩展的多个独立服务:
* 流推送服务，用于处理实时流推送.</br>
* 媒体数据交换中心处理混合来源.</br>
* 控制中心，为多个水平服务提供负载平衡和群集管理.</br>
* 静态文件和直播回放服务.</br>
* 多协议直播流推送处理服务.</br>


### 计划
> 早期计划仅支持RTMP，WebRTC，HttpFLV协议.</br>

* [x] rtmp推送流处理</br>
* [x] 流交换中心</br>
* [x] 负载均衡服务</br>
* [ ] 音视频数据处理</br>
* [x] 直播服务</br>
* [ ] 直播回放和静态文件支持</br>
* [ ] WebRTC TURN支持</br>


### 展望
* 推流SDK.</br>
* 自主开发的流协议.</br>
* 支持尽可能多的现有协议.</br>
* 尽可能好的表现.</br>
* 自适应多重编码.</br>


### License
[GPL](./LICENSE)
Copyright (c) 2020 Mr.Panda.
