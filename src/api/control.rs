use axum::{Router, routing::post, AddExtensionLayer};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::partitioning::get_partition;

pub struct ControlNode {
    workers: Vec<String>,
}

impl ControlNode {
    pub fn new(workers: Vec<String>) -> Self {
        Self { workers }
    }
}

pub async fn start_control_node() {
    let workers = vec!["http://localhost:8081".to_string(), "http://localhost:8082".to_string()];
    let state = Arc::new(Mutex::new(ControlNode::new(workers)));

    let app = Router::new()
        .route("/insert", post(insert_handler))
        .route("/analytics", post(analytics_handler))
        .layer(AddExtensionLayer::new(state));

    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn insert_handler() {
    // Implementation to route data based on partition key
}

async fn analytics_handler() {
    // Implementation to collect and aggregate data from worker nodes
}
