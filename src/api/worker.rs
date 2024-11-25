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
use chrono;

use crate::{
    Result,
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
pub struct MetricRequest {
    pub metric_name: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerMetricResponse {
    pub name: String,
    pub value: f64,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricAggregateResponse {
    pub name: String,
    pub count: u64,
    pub sum: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
}

pub fn worker_router(state: WorkerState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/process", post(process_metric))
        .route("/metrics/:name", get(get_metric))
        .route("/metrics/:name/aggregate", get(get_metric_aggregate))
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
    Json(request): Json<MetricRequest>,
) -> Result<Json<WorkerMetricResponse>> {
    info!(
        "Worker {} processing metric: {} = {}",
        state.worker_id, request.metric_name, request.value
    );
    
    state.metrics.record_metric(&request.metric_name, request.value).await?;
    
    Ok(Json(WorkerMetricResponse {
        name: request.metric_name,
        value: request.value,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

async fn get_metric(
    State(state): State<WorkerState>,
    Path(name): Path<String>,
) -> Result<Json<WorkerMetricResponse>> {
    info!("Worker {} retrieving metric: {}", state.worker_id, name);
    
    let value = state.metrics.get_metric(&name).await?
        .ok_or(RaftMetricsError::NotFound)?;
    
    Ok(Json(WorkerMetricResponse {
        name: name.clone(),
        value,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

async fn get_metric_aggregate(
    State(state): State<WorkerState>,
    Path(name): Path<String>,
) -> Result<Json<MetricAggregateResponse>> {
    info!("Worker {} calculating aggregate for metric: {}", state.worker_id, name);
    
    let aggregate = state.metrics.get_metric_aggregate(&name).await?
        .ok_or(RaftMetricsError::NotFound)?;
    
    Ok(Json(MetricAggregateResponse {
        name: name.clone(),
        count: aggregate.count,
        sum: aggregate.sum,
        average: aggregate.average,
        min: aggregate.min,
        max: aggregate.max,
    }))
}

pub async fn start_worker_node(worker_id: usize) {
    let storage = Arc::new(MemStorage::new());
    let metrics = Arc::new(MetricsRegistry::new());

    let state = WorkerState {
        storage: storage.clone(),
        metrics: metrics.clone(),
        worker_id,
    };

    let port = env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("Starting worker node {} on {}", worker_id, addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, worker_router(state)).await.unwrap();
}
