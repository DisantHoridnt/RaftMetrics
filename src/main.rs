use std::env;
use tracing::info;

#[tokio::main]
async fn main() {
    // Get node type from environment variable
    let node_type = env::var("NODE_TYPE").unwrap_or_else(|_| "control".to_string());
    
    // Try WORKER_ID first, then NODE_ID for backward compatibility
    let worker_id: usize = env::var("NODE_ID")
        .or_else(|_| env::var("WORKER_ID"))
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .unwrap_or(1);

    // Initialize logging
    distributed_analytics_system::logging::init_logger(worker_id as u64, &node_type);

    info!("Starting node with type: {}", node_type);
    
    match node_type.as_str() {
        "worker" => {
            info!("Starting worker node {}", worker_id);
            distributed_analytics_system::api::worker::start_worker_node(worker_id).await;
        }
        _ => {
            info!("Starting control node");
            distributed_analytics_system::api::control::start_control_node().await;
        }
    }
}
