use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use sqlx::{sqlite::SqlitePool, Row};

use crate::service::{NodeInfo, NodeInfoWithHeartbeat};

/// 数据库操作封装
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// 创建数据库连接
    pub async fn new(database_path: &str) -> Result<Self> {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        Ok(Self { pool })
    }

    /// 初始化数据库表结构
    pub async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS nodes (
                node_id TEXT PRIMARY KEY,
                address TEXT NOT NULL,
                port_range_start INTEGER NOT NULL,
                port_range_end INTEGER NOT NULL,
                allocated_ports INTEGER NOT NULL DEFAULT 0,
                capacity INTEGER NOT NULL,
                last_heartbeat INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_nodes_heartbeat 
            ON nodes(last_heartbeat)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_nodes_port_range 
            ON nodes(port_range_start, port_range_end)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 注册或更新节点
    pub async fn upsert_node(&self, node: &NodeInfo) -> Result<()> {
        let now = current_timestamp();

        sqlx::query(
            r#"
            INSERT INTO nodes (
                node_id, address, port_range_start, port_range_end,
                allocated_ports, capacity, last_heartbeat, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(node_id) DO UPDATE SET
                address = excluded.address,
                port_range_start = excluded.port_range_start,
                port_range_end = excluded.port_range_end,
                allocated_ports = excluded.allocated_ports,
                capacity = excluded.capacity,
                last_heartbeat = excluded.last_heartbeat
            "#,
        )
        .bind(&node.node_id)
        .bind(node.address.to_string())
        .bind(node.port_range_start as i64)
        .bind(node.port_range_end as i64)
        .bind(node.allocated_ports as i64)
        .bind(node.capacity as i64)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 更新节点心跳
    pub async fn update_heartbeat(&self, node_id: &str) -> Result<()> {
        let now = current_timestamp();

        sqlx::query(
            r#"
            UPDATE nodes 
            SET last_heartbeat = ? 
            WHERE node_id = ?
            "#,
        )
        .bind(now)
        .bind(node_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 更新节点端口使用情况
    pub async fn update_allocated_ports(&self, node_id: &str, allocated: u32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE nodes 
            SET allocated_ports = ? 
            WHERE node_id = ?
            "#,
        )
        .bind(allocated as i64)
        .bind(node_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 获取所有活跃节点
    pub async fn get_active_nodes(&self, timeout: Duration) -> Result<Vec<NodeInfoWithHeartbeat>> {
        let threshold = current_timestamp() - timeout.as_secs() as i64;

        let rows = sqlx::query(
            r#"
            SELECT 
                node_id, address, port_range_start, port_range_end,
                allocated_ports, capacity, last_heartbeat
            FROM nodes
            WHERE last_heartbeat >= ?
            ORDER BY allocated_ports ASC, capacity DESC
            "#,
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await?;

        let mut nodes = Vec::new();
        for row in rows {
            let node = NodeInfo {
                node_id: row.get("node_id"),
                address: row.get::<String, _>("address").parse()?,
                port_range_start: row.get::<i64, _>("port_range_start") as u16,
                port_range_end: row.get::<i64, _>("port_range_end") as u16,
                allocated_ports: row.get::<i64, _>("allocated_ports") as u32,
                capacity: row.get::<i64, _>("capacity") as u32,
            };
            nodes.push(NodeInfoWithHeartbeat {
                node,
                last_heartbeat: row.get::<i64, _>("last_heartbeat"),
            });
        }

        Ok(nodes)
    }

    /// 根据端口查找节点
    ///
    /// 用于跨节点转发时查找目标端口所在的节点
    pub async fn find_node_by_port(&self, port: u16) -> Result<Option<NodeInfoWithHeartbeat>> {
        let port = port as i64;

        let row = sqlx::query(
            r#"
            SELECT 
                node_id, address, port_range_start, port_range_end,
                allocated_ports, capacity, last_heartbeat
            FROM nodes
            WHERE ? >= port_range_start AND ? <= port_range_end
            LIMIT 1
            "#,
        )
        .bind(port)
        .bind(port)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let node = NodeInfo {
                node_id: row.get("node_id"),
                address: row.get::<String, _>("address").parse()?,
                port_range_start: row.get::<i64, _>("port_range_start") as u16,
                port_range_end: row.get::<i64, _>("port_range_end") as u16,
                allocated_ports: row.get::<i64, _>("allocated_ports") as u32,
                capacity: row.get::<i64, _>("capacity") as u32,
            };
            Ok(Some(NodeInfoWithHeartbeat {
                node,
                last_heartbeat: row.get::<i64, _>("last_heartbeat"),
            }))
        } else {
            Ok(None)
        }
    }

    /// 清理超时节点
    ///
    /// 删除心跳时间超过指定阈值的节点记录。
    /// 当节点断开连接或停止发送心跳时，会被自动清理。
    pub async fn cleanup_timeout_nodes(&self, timeout: Duration) -> Result<usize> {
        let threshold = current_timestamp() - timeout.as_secs() as i64;

        let result = sqlx::query(
            r#"
            DELETE FROM nodes
            WHERE last_heartbeat < ?
            "#,
        )
        .bind(threshold)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    /// 获取节点信息
    pub async fn get_node(&self, node_id: &str) -> Result<Option<NodeInfo>> {
        let row = sqlx::query(
            r#"
            SELECT 
                node_id, address, port_range_start, port_range_end,
                allocated_ports, capacity, last_heartbeat
            FROM nodes
            WHERE node_id = ?
            "#,
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let node = NodeInfo {
                node_id: row.get("node_id"),
                address: row.get::<String, _>("address").parse()?,
                port_range_start: row.get::<i64, _>("port_range_start") as u16,
                port_range_end: row.get::<i64, _>("port_range_end") as u16,
                allocated_ports: row.get::<i64, _>("allocated_ports") as u32,
                capacity: row.get::<i64, _>("capacity") as u32,
            };
            Ok(Some(node))
        } else {
            Ok(None)
        }
    }

    /// 获取所有节点（包括非活跃节点）
    pub async fn get_all_nodes(&self) -> Result<Vec<NodeInfoWithHeartbeat>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                node_id, address, port_range_start, port_range_end,
                allocated_ports, capacity, last_heartbeat
            FROM nodes
            ORDER BY allocated_ports ASC, capacity DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut nodes = Vec::new();
        for row in rows {
            let node = NodeInfo {
                node_id: row.get("node_id"),
                address: row.get::<String, _>("address").parse()?,
                port_range_start: row.get::<i64, _>("port_range_start") as u16,
                port_range_end: row.get::<i64, _>("port_range_end") as u16,
                allocated_ports: row.get::<i64, _>("allocated_ports") as u32,
                capacity: row.get::<i64, _>("capacity") as u32,
            };
            nodes.push(NodeInfoWithHeartbeat {
                node,
                last_heartbeat: row.get::<i64, _>("last_heartbeat"),
            });
        }

        Ok(nodes)
    }
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

