use std::env;
use tracing::info;
use distributed_analytics_system::api::worker;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get worker ID from environment variable
    let worker_id: usize = env::var("WORKER_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .unwrap_or(1);

    info!("Starting worker node {}", worker_id);
    worker::start_worker_node(worker_id).await;
}
