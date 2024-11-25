mod api;
mod metrics;
mod raft;
mod partitioning;

use std::env;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub type Result<T> = std::result::Result<T, RaftMetricsError>;

#[derive(Debug, thiserror::Error)]
pub enum RaftMetricsError {
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Not found")]
    NotFound(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    
    #[error(transparent)]
    Raft(#[from] raft::Error),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .with_thread_names(true)
        .with_ansi(true)
        .pretty()
        .init();

    info!("Starting RaftMetrics");

    let node_type = env::var("NODE_TYPE").unwrap_or_else(|_| "worker".to_string());
    let node_id = env::var("NODE_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<usize>()
        .unwrap();

    match node_type.as_str() {
        "control" => {
            info!("Starting control node");
            api::control::start_control_node().await;
        }
        "worker" => {
            info!("Starting worker node {}", node_id);
            api::worker::start_worker_node(node_id).await;
        }
        _ => {
            panic!("Invalid node type: {}", node_type);
        }
    }

    Ok(())
}
