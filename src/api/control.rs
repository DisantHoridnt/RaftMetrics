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
pub struct ControlState {
    pub storage: Arc<MemStorage>,
    pub metrics: Arc<MetricsRegistry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricRequest {
    pub metric_name: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricResponse {
    pub success: bool,
    pub message: String,
}

pub fn control_router(state: ControlState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", post(record_metric))
        .route("/metrics/:name", get(get_metric))
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "message": "Control node is operational"
    }))
}

async fn record_metric(
    State(state): State<ControlState>,
    Json(request): Json<MetricRequest>,
) -> Result<impl IntoResponse, RaftMetricsError> {
    info!("Recording metric: {} = {}", request.metric_name, request.value);
    
    state.metrics.record_metric(&request.metric_name, request.value).await?;
    
    Ok(Json(MetricResponse {
        success: true,
        message: format!("Metric {} recorded successfully", request.metric_name),
    }))
}

async fn get_metric(
    State(state): State<ControlState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, RaftMetricsError> {
    info!("Retrieving metric: {}", name);
    
    let value = state.metrics.get_metric(&name).await?;
    
    Ok(Json(serde_json::json!({
        "metric_name": name,
        "value": value
    })))
}

pub async fn start_control_node() {
    let storage = Arc::new(MemStorage::new());
    let metrics = Arc::new(MetricsRegistry::new());
    
    let state = ControlState {
        storage,
        metrics,
    };
    
    let app = control_router(state);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Starting control node on {}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
