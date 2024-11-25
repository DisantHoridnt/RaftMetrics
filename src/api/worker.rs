use axum::{
    extract::{State, Path},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use tokio::net::TcpListener;
use std::env;

use crate::{
    RaftMetricsError,
    metrics::MetricsRegistry,
    raft::storage::MemStorage,
};

#[derive(Clone)]
pub struct WorkerState {
    pub storage: Arc<MemStorage>,
    pub metrics: Arc<MetricsRegistry>,
    pub worker_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessRequest {
    pub metric_name: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricResponse {
    pub metric_name: String,
    pub value: f64,
    pub worker_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateResponse {
    pub metric_name: String,
    pub count: usize,
    pub sum: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
    pub worker_id: usize,
}

pub fn worker_router(state: WorkerState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/process", post(process_metric))
        .route("/metrics/:name", get(get_metric))
        .route("/metrics/:name/aggregate", get(get_metric_aggregate))
        .with_state(state)
}

async fn health_check(State(state): State<WorkerState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "message": format!("Worker node {} is operational", state.worker_id),
    }))
}

async fn process_metric(
    State(state): State<WorkerState>,
    Json(request): Json<ProcessRequest>,
) -> Result<impl IntoResponse, RaftMetricsError> {
    info!(
        "Worker {} processing metric: {} = {}",
        state.worker_id, request.metric_name, request.value
    );
    
    state.metrics.record_metric(&request.metric_name, request.value).await?;
    
    Ok(Json(ProcessResponse {
        success: true,
        message: format!("Metric {} processed successfully by worker {}", request.metric_name, state.worker_id),
    }))
}

async fn get_metric(
    State(state): State<WorkerState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, RaftMetricsError> {
    info!("Worker {} retrieving metric: {}", state.worker_id, name);
    
    let value = state.metrics.get_metric(&name).await?;
    
    Ok(Json(MetricResponse {
        metric_name: name,
        value,
        worker_id: state.worker_id,
    }))
}

async fn get_metric_aggregate(
    State(state): State<WorkerState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, RaftMetricsError> {
    info!("Worker {} calculating aggregate for metric: {}", state.worker_id, name);
    
    let aggregate = state.metrics.get_metric_aggregate(&name).await?;
    
    Ok(Json(AggregateResponse {
        metric_name: name,
        count: aggregate.count,
        sum: aggregate.sum,
        average: aggregate.average,
        min: aggregate.min,
        max: aggregate.max,
        worker_id: state.worker_id,
    }))
}

pub async fn start_worker_node(worker_id: usize) {
    let storage = Arc::new(MemStorage::new());
    let metrics = Arc::new(MetricsRegistry::new());
    
    let state = WorkerState {
        storage,
        metrics,
        worker_id,
    };
    
    let app = worker_router(state);
    
    let port = env::var("PORT").unwrap_or_else(|_| {
        match worker_id {
            1 => "8081",
            2 => "8082",
            _ => "8081",
        }.to_string()
    });
    
    let addr = format!("0.0.0.0:{}", port);
    info!("Starting worker node {} on {}", worker_id, addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
