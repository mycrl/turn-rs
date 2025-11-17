use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
use tonic::{Request, Response, Status};

use crate::database::Database;

use protos::cluster::{
    cluster_router_service_server::ClusterRouterService,
    ClusterInfo, FindNodeByPortRequest, FindNodeByPortResponse, GetNodesRequest, GetNodesResponse,
    NodeInfo as ProtoNodeInfo, RegisterNodeRequest, RegisterNodeResponse, UpdateNodeStatusRequest,
};

/// 节点信息（内部使用）
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub node_id: String,
    pub address: SocketAddr,
    pub port_range_start: u16,
    pub port_range_end: u16,
    pub allocated_ports: u32,
    pub capacity: u32,
}

/// 带心跳时间的节点信息（用于数据库查询）
pub struct NodeInfoWithHeartbeat {
    pub node: NodeInfo,
    pub last_heartbeat: i64,
}

impl From<NodeInfo> for ProtoNodeInfo {
    fn from(node: NodeInfo) -> Self {
        ProtoNodeInfo {
            node_id: node.node_id,
            address: node.address.to_string(),
            port_range_start: node.port_range_start as u32,
            port_range_end: node.port_range_end as u32,
            allocated_ports: node.allocated_ports,
            capacity: node.capacity,
            last_heartbeat: 0, // 将在查询时从数据库填充
        }
    }
}

impl From<NodeInfoWithHeartbeat> for ProtoNodeInfo {
    fn from(node_with_heartbeat: NodeInfoWithHeartbeat) -> Self {
        let mut proto: ProtoNodeInfo = node_with_heartbeat.node.into();
        proto.last_heartbeat = node_with_heartbeat.last_heartbeat;
        proto
    }
}

/// 集群路由服务实现
pub struct ClusterRouterServiceImpl {
    db: Database,
}

impl ClusterRouterServiceImpl {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[tonic::async_trait]
impl ClusterRouterService for ClusterRouterServiceImpl {
    /// 注册节点
    async fn register_node(
        &self,
        request: Request<RegisterNodeRequest>,
    ) -> Result<Response<RegisterNodeResponse>, Status> {
        let req = request.into_inner();

        // 验证端口范围
        if req.port_range_start > req.port_range_end {
            return Ok(Response::new(RegisterNodeResponse {
                success: false,
                message: "Invalid port range: start > end".to_string(),
            }));
        }

        // 解析地址
        let address = req.address.parse::<SocketAddr>().map_err(|e| {
            Status::invalid_argument(format!("Invalid address format: {}", e))
        })?;

        let capacity = (req.port_range_end - req.port_range_start + 1) as u32;

        // 获取当前分配的端口数（如果节点已存在）
        let allocated = self
            .db
            .get_node(&req.node_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .map(|n| n.allocated_ports)
            .unwrap_or(0);

        let node = NodeInfo {
            node_id: req.node_id.clone(),
            address,
            port_range_start: req.port_range_start as u16,
            port_range_end: req.port_range_end as u16,
            allocated_ports: allocated,
            capacity,
        };

        // 注册节点
        self.db
            .upsert_node(&node)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // 更新心跳
        self.db
            .update_heartbeat(&req.node_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        log::info!(
            "Registered node: {} at {} (port range: {}..{})",
            req.node_id,
            address,
            req.port_range_start,
            req.port_range_end
        );

        Ok(Response::new(RegisterNodeResponse {
            success: true,
            message: "Node registered successfully".to_string(),
        }))
    }

    /// 更新节点状态
    async fn update_node_status(
        &self,
        request: Request<UpdateNodeStatusRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        // 更新心跳
        self.db
            .update_heartbeat(&req.node_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // 更新端口使用情况（如果提供）
        if let Some(allocated) = req.allocated_ports {
            self.db
                .update_allocated_ports(&req.node_id, allocated)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
        }

        Ok(Response::new(()))
    }

    /// 根据端口查找节点
    ///
    /// 只返回活跃节点。超时节点会被后台任务自动清理。
    async fn find_node_by_port(
        &self,
        request: Request<FindNodeByPortRequest>,
    ) -> Result<Response<FindNodeByPortResponse>, Status> {
        let req = request.into_inner();
        let port = req.port as u16;

        // 查找节点（只返回数据库中的节点，超时节点会被后台任务清理）
        let node = self
            .db
            .find_node_by_port(port)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(FindNodeByPortResponse {
            node: node.map(|n| n.into()),
        }))
    }

    /// 获取所有节点
    async fn get_nodes(
        &self,
        request: Request<GetNodesRequest>,
    ) -> Result<Response<GetNodesResponse>, Status> {
        let req = request.into_inner();

        let nodes = if req.active_only {
            // 只返回活跃节点（心跳在60秒内，与清理任务的超时时间一致）
            // 超时节点会被后台任务自动清理，所以这里查询到的都是活跃节点
            self.db
                .get_active_nodes(Duration::from_secs(60))
                .await
                .map_err(|e| Status::internal(e.to_string()))?
        } else {
            // 返回所有节点（包括非活跃节点，但超时节点会被后台任务清理）
            self.db
                .get_all_nodes()
                .await
                .map_err(|e| Status::internal(e.to_string()))?
        };

        Ok(Response::new(GetNodesResponse {
            nodes: nodes.into_iter().map(|n| n.into()).collect(),
        }))
    }

    /// 获取集群信息
    ///
    /// 只统计活跃节点的信息。超时节点会被后台任务自动清理。
    async fn get_cluster_info(
        &self,
        _request: Request<()>,
    ) -> Result<Response<ClusterInfo>, Status> {
        // 获取活跃节点（心跳在60秒内，与清理任务的超时时间一致）
        let active_nodes = self
            .db
            .get_active_nodes(Duration::from_secs(60))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let total_capacity: u32 = active_nodes.iter().map(|n| n.node.capacity).sum();
        let total_allocated: u32 = active_nodes.iter().map(|n| n.node.allocated_ports).sum();
        let active_count = active_nodes.len() as u32;

        // 获取总节点数（包括非活跃节点）
        let all_nodes = self
            .db
            .get_all_nodes()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let total_nodes = all_nodes.len() as u32;

        Ok(Response::new(ClusterInfo {
            total_capacity,
            total_allocated,
            active_nodes: active_count,
            total_nodes,
        }))
    }
}
