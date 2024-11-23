use axum::{
    extract::{State, Path},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use tokio::net::TcpListener;

use crate::{
    RaftMetricsError,
    metrics::MetricsRegistry,
    raft::storage::MemStorage,
};

#[derive(Clone)]
pub struct WorkerState {
    pub storage: Arc<MemStorage>,
    pub metrics: Arc<MetricsRegistry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessRequest {
    pub metric_name: String,
    pub value: f64,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessResponse {
    pub success: bool,
    pub message: String,
}

pub fn worker_router(state: WorkerState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/process", post(process_metric))
        .route("/metrics/:name", get(get_metric))
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "message": "Worker node is operational"
    }))
}

async fn process_metric(
    State(state): State<WorkerState>,
    Json(request): Json<ProcessRequest>,
) -> Result<impl IntoResponse, RaftMetricsError> {
    info!(
        "Processing metric: {} = {} at {}",
        request.metric_name, request.value, request.timestamp
    );
    
    state.metrics.record_metric_with_timestamp(
        &request.metric_name,
        request.value,
        request.timestamp,
    ).await?;
    
    Ok(Json(ProcessResponse {
        success: true,
        message: format!("Metric {} processed successfully", request.metric_name),
    }))
}

async fn get_metric(
    State(state): State<WorkerState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, RaftMetricsError> {
    info!("Retrieving metric: {}", name);
    
    let value = state.metrics.get_metric(&name).await?;
    
    Ok(Json(serde_json::json!({
        "metric_name": name,
        "value": value
    })))
}

pub async fn start_worker_node() {
    let storage = Arc::new(MemStorage::new());
    let metrics = Arc::new(MetricsRegistry::new());
    
    let state = WorkerState {
        storage,
        metrics,
    };
    
    let app = worker_router(state);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    info!("Starting worker node on {}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
