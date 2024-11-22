use std::env;
use raft_metrics::{
    api::{control, worker},
    logging,
    metrics,
};

#[tokio::main]
async fn main() {
    // Initialize logging
    let node_role = env::var("NODE_ROLE").unwrap_or_else(|_| "control".to_string());
    let node_id = env::var("NODE_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .unwrap_or(1);

    let logger = logging::setup_logger(node_id, &node_role);

    // Initialize metrics
    metrics::init_metrics();

    // Start node based on role
    match node_role.as_str() {
        "control" => {
            control::start_control_node(logger).await;
        }
        "worker" => {
            worker::start_worker_node(logger).await;
        }
        _ => {
            eprintln!("Invalid NODE_ROLE. Must be 'control' or 'worker'");
            std::process::exit(1);
        }
    }
}
