# Quasipaa

这是一个主要使用Rust编程语言构建的大规模高可用性和高性能流媒体集群，这是新的尝试和良好的开端.
希望依靠Rust的高性能和优雅的设计在流媒体领域中越走越远.


### 设计
> 包括推流处理，媒体协议处理，实时点播服务，并包括为多服务集群提供高可用性的负载均衡服务.

```rust
                                                        +------+
                                                        | core |
                                                        +------+
                                                            |
                                           /------------------------------\
                               
                                        +---------+    +----------+    +------+
                               push ->  | publish | -> | exchange | -> | pull | -> player
                                        +---------+    +----------+    +------+
```


### 版本
开发阶段 </br>
项目进度更新在 [项目看板](https://github.com/quasipaas/Quasipaa/projects/1)，可以随时跟踪.</br>


### 支持的编码器

* [x] H264</br>
* [ ] H265</br>
* [ ] VP9</br>
* [ ] AV1</br>
* [x] AAC</br>
* [ ] MP3</br>
* [ ] WAV</br>


### 支持的协议

* [x] HTTP-FLV</br>
* [x] RTMP</br>
* [ ] RTSP</br>
* [ ] WebRTC</br>
* [ ] HLS</br>
* [ ] DASH FMP4</br>


### 计划
> 早期计划仅支持RTMP，HLS，HttpFLV协议，目前仅考虑H264编码，在接下来的计划中支持WebRTC和VP9.</br>

* [x] rtmp推送流处理</br>
* [x] 流交换中心</br>
* [x] 负载均衡服务</br>
* [ ] 音视频数据处理</br>
* [x] 直播服务</br>
* [ ] 直播回放和静态文件支持</br>


### 概述
Quasipaa是使用Rust编程语言编写的流媒体服务集群，其中包括可以水平扩展的多个独立服务:
* 流推送服务，用于处理实时流推送.</br>
* 媒体数据交换中心处理混合来源.</br>
* 控制中心，为多个水平服务提供负载平衡和群集管理.</br>
* 静态文件和直播回放服务.</br>
* 多协议直播流推送处理服务.</br>


### 展望
* 推流SDK.</br>
* 自主开发的流协议.</br>
* 支持尽可能多的现有协议.</br>
* 尽可能好的表现.</br>
* WebRTC的TURN支持.</br>
* 自适应多重编码.</br>


### License
[GPL](./LICENSE)
Copyright (c) 2020 Mr.Panda.
