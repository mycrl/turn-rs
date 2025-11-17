# TURN 集群路由服务

集群路由服务负责管理 TURN 服务器集群中所有节点的状态，提供节点查询和负载均衡功能。

## 功能特性

-   ✅ 节点注册和心跳管理
-   ✅ 节点状态存储（SQLite）
-   ✅ 端口范围查询
-   ✅ 节点负载统计
-   ✅ 超时节点自动清理
-   ✅ 独立的 gRPC API（`ClusterRouterService`）
-   ✅ 根据端口查找节点（用于跨节点转发）

## 使用方法

### 启动服务

```bash
cargo run --bin turn-cluster-router -- \
    --database cluster-router.db \
    --listen 127.0.0.1:3001 \
    --heartbeat-timeout 60 \
    --cleanup-interval 10
```

### 配置参数

-   `--database, -d`: 数据库文件路径（默认: `cluster-router.db`）
-   `--listen, -l`: gRPC 服务监听地址（默认: `127.0.0.1:3001`）
-   `--heartbeat-timeout`: 节点心跳超时时间，秒（默认: 60）。超过此时间未更新心跳的节点将被自动清理
-   `--cleanup-interval`: 清理任务执行间隔，秒（默认: 10）。后台任务每隔此时间检查并清理超时节点

## API 接口

### 1. RegisterNode - 注册节点

节点启动时调用此接口注册到集群。

```rust
use protos::cluster::cluster_router_service_client::ClusterRouterServiceClient;
use protos::cluster::RegisterNodeRequest;

let mut client = ClusterRouterServiceClient::connect("http://127.0.0.1:3001").await?;

let request = RegisterNodeRequest {
    node_id: "node-1".to_string(),
    address: "192.168.1.1:3478".to_string(),
    port_range_start: 49152,
    port_range_end: 50000,
};

let response = client.register_node(request).await?;
```

### 2. UpdateNodeStatus - 更新节点状态

节点定期调用此接口更新心跳和端口使用情况。

```rust
use protos::cluster::UpdateNodeStatusRequest;

let request = UpdateNodeStatusRequest {
    node_id: "node-1".to_string(),
    allocated_ports: Some(100), // 可选：更新端口使用数
};

client.update_node_status(request).await?;
```

### 3. FindNodeByPort - 根据端口查找节点

**这是关键接口**：当节点收到一个不属于自己端口范围的请求时，调用此接口查找目标节点。

```rust
use protos::cluster::FindNodeByPortRequest;

let request = FindNodeByPortRequest {
    port: 51000, // 目标端口
};

let response = client.find_node_by_port(request).await?;
if let Some(node) = response.into_inner().node {
    println!("Target node: {} at {}", node.node_id, node.address);
    // 将数据包转发到 node.address
}
```

### 4. GetNodes - 获取节点列表

获取所有节点或仅活跃节点列表（用于负载均衡）。

```rust
use protos::cluster::GetNodesRequest;

let request = GetNodesRequest {
    active_only: true, // 只返回活跃节点
};

let response = client.get_nodes(request).await?;
for node in response.into_inner().nodes {
    println!("Node: {} - Capacity: {}/{}",
        node.node_id,
        node.allocated_ports,
        node.capacity
    );
}
```

### 5. GetClusterInfo - 获取集群信息

获取集群汇总信息。

```rust
use protos::cluster::GetClusterInfoRequest;

let response = client.get_cluster_info(()).await?;
let info = response.into_inner();
println!("Total capacity: {}", info.total_capacity);
println!("Total allocated: {}", info.total_allocated);
println!("Active nodes: {}", info.active_nodes);
```

## 使用场景

### 场景 1：节点注册

```rust
// 节点启动时
let register_request = RegisterNodeRequest {
    node_id: "node-1".to_string(),
    address: "192.168.1.1:3478".to_string(),
    port_range_start: 49152,
    port_range_end: 50000,
};
client.register_node(register_request).await?;
```

### 场景 2：跨节点转发

```rust
// 节点收到数据包，目标端口不在本地范围
let target_port = 51000;
if !is_local_port(target_port) {
    // 查询目标节点
    let find_request = FindNodeByPortRequest { port: target_port };
    let response = client.find_node_by_port(find_request).await?;

    if let Some(node) = response.into_inner().node {
        // 通过UDP转发到目标节点的内部地址
        forward_to_node(node.address, data).await?;
    }
}
```

### 场景 3：定期心跳

```rust
// 每30秒更新一次心跳
loop {
    tokio::time::sleep(Duration::from_secs(30)).await;

    let update_request = UpdateNodeStatusRequest {
        node_id: "node-1".to_string(),
        allocated_ports: Some(current_allocated_ports), // 可选
    };
    client.update_node_status(update_request).await?;
}
```

## 数据库结构

```sql
CREATE TABLE nodes (
    node_id TEXT PRIMARY KEY,
    address TEXT NOT NULL,
    port_range_start INTEGER NOT NULL,
    port_range_end INTEGER NOT NULL,
    allocated_ports INTEGER NOT NULL DEFAULT 0,
    capacity INTEGER NOT NULL,
    last_heartbeat INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_nodes_heartbeat ON nodes(last_heartbeat);
CREATE INDEX idx_nodes_port_range ON nodes(port_range_start, port_range_end);
```

## 心跳机制和自动清理

### 心跳更新

-   节点注册时自动更新心跳时间
-   调用 `UpdateNodeStatus` 时自动更新心跳时间
-   每次调用任何接口时，如果请求中包含节点 ID，都会更新心跳时间

### 自动清理机制

-   **后台清理任务**：每隔 `--cleanup-interval` 秒（默认 10 秒）执行一次
-   **超时检测**：检查所有节点的心跳时间，如果超过 `--heartbeat-timeout` 秒（默认 60 秒）未更新，则自动删除该节点记录
-   **日志记录**：每次清理超时节点时，会记录清理的节点数量

### 工作原理

```
节点正常 → 定期调用 UpdateNodeStatus → 更新心跳时间 → 不会被清理
         ↓
节点断开 → 停止调用 UpdateNodeStatus → 心跳时间不更新
         ↓
后台任务检测 → 心跳超过60秒 → 自动删除数据库记录
```

### 配置建议

-   **心跳超时时间**：建议设置为节点心跳间隔的 2-3 倍
    -   如果节点每 30 秒发送一次心跳，建议设置为 60-90 秒
-   **清理间隔**：建议设置为心跳超时时间的 1/6 到 1/3
    -   如果超时时间为 60 秒，建议设置为 10-20 秒

## 未来计划

-   [ ] 添加节点认证机制
-   [ ] 支持节点元数据（标签、区域等）
-   [ ] 支持节点负载预测
-   [ ] 支持节点健康检查
