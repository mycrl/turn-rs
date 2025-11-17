use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use tokio::time::interval;

mod database;
mod service;

use database::Database;
use service::ClusterRouterServiceImpl;

use protos::cluster::cluster_router_service_server::ClusterRouterServiceServer;

/// TURN集群路由服务
///
/// 负责管理集群中所有节点的状态，提供节点查询和负载均衡功能
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 数据库文件路径
    #[arg(short, long, default_value = "cluster-router.db")]
    database: String,

    /// gRPC服务监听地址
    #[arg(short, long, default_value = "127.0.0.1:3001")]
    listen: SocketAddr,

    /// 节点心跳超时时间（秒），超过此时间未更新心跳的节点将被自动清理
    #[arg(long, default_value = "60")]
    heartbeat_timeout: u64,

    /// 清理任务执行间隔（秒）
    #[arg(long, default_value = "10")]
    cleanup_interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 初始化日志
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()?;

    log::info!("Starting TURN cluster router...");
    log::info!("Database: {}", args.database);
    log::info!("Listen address: {}", args.listen);

    // 初始化数据库
    let db = Database::new(&args.database).await?;
    db.init_schema().await?;
    log::info!("Database initialized");

    // 创建服务
    let router_service = ClusterRouterServiceImpl::new(db.clone());

    // 启动后台清理任务：定期清理超时节点
    let db_cleanup = db.clone();
    let timeout = Duration::from_secs(args.heartbeat_timeout);
    let cleanup_interval = Duration::from_secs(args.cleanup_interval);
    tokio::spawn(async move {
        let mut interval = interval(cleanup_interval);
        loop {
            interval.tick().await;
            match db_cleanup.cleanup_timeout_nodes(timeout).await {
                Ok(count) => {
                    if count > 0 {
                        log::info!("Cleaned up {} timeout nodes (heartbeat timeout: {}s)", count, timeout.as_secs());
                    }
                }
                Err(e) => {
                    log::error!("Failed to cleanup timeout nodes: {}", e);
                }
            }
        }
    });

    log::info!("Cleanup task started: interval={}s, timeout={}s", args.cleanup_interval, args.heartbeat_timeout);

    // 启动gRPC服务器
    tonic::transport::Server::builder()
        .add_service(ClusterRouterServiceServer::new(router_service))
        .serve(args.listen)
        .await?;

    Ok(())
}
