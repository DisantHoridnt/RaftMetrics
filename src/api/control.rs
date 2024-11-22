use axum::{Router, routing::post, extract::Json, response::IntoResponse, http::StatusCode};
use std::{sync::Arc, env};
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use crate::partitioning::get_partition;

#[derive(Debug, Deserialize)]
pub struct InsertRequest {
    partition_key: String,
    value: f64,
}

pub struct ControlNode {
    workers: Vec<String>,
}

impl ControlNode {
    pub fn new() -> Self {
        let worker_hosts = env::var("WORKER_HOSTS")
            .unwrap_or_else(|_| "worker1:8081,worker2:8082".to_string());
        
        let workers = worker_hosts
            .split(',')
            .map(|host| format!("http://{}", host.trim()))
            .collect();

        Self { workers }
    }

    async fn route_request(&self, key: &str, value: f64) -> Result<(), Box<dyn std::error::Error>> {
        let partition = get_partition(key, self.workers.len());
        let worker_url = &self.workers[partition];
        
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/insert", worker_url))
            .json(&InsertRequest {
                partition_key: key.to_string(),
                value,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err("Worker request failed".into());
        }

        Ok(())
    }
}

pub async fn start_control_node() {
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let state = Arc::new(Mutex::new(ControlNode::new()));

    let app = Router::new()
        .route("/insert", post(insert_handler))
        .route("/analytics", post(analytics_handler))
        .with_state(state);

    println!("Control node starting on port {}", port);

    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn insert_handler(
    state: axum::extract::State<Arc<Mutex<ControlNode>>>,
    Json(request): Json<InsertRequest>,
) -> impl IntoResponse {
    let node = state.lock().await;
    match node.route_request(&request.partition_key, request.value).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn analytics_handler() -> impl IntoResponse {
    // Implementation to collect and aggregate data from worker nodes
    StatusCode::NOT_IMPLEMENTED
}
