use axum::{
    extract::{State, Path},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::info;
use tokio::net::TcpListener;
use std::env;
use chrono;

use crate::{
    Result,
    RaftMetricsError,
    metrics::MetricsRegistry,
    raft::{
        node::{RaftNode, run_raft_node},
        storage::MemStorage,
    },
};

#[derive(Clone)]
pub struct WorkerState {
    pub storage: Arc<MemStorage>,
    pub metrics: Arc<MetricsRegistry>,
    pub worker_id: usize,
    pub proposal_tx: mpsc::Sender<Vec<u8>>,
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

fn worker_router(state: WorkerState) -> Router {
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
    info!("Worker {} processing metric: {} = {}", 
        state.worker_id, request.metric_name, request.value);

    // Create proposal data
    let data = serde_json::to_vec(&request)
        .map_err(|e| RaftMetricsError::Internal(format!("Failed to serialize metric: {}", e)))?;

    // Send proposal through Raft
    state.proposal_tx.send(data).await.map_err(|e| {
        RaftMetricsError::Internal(format!("Failed to send proposal: {}", e))
    })?;

    // For now, still process locally
    // This will be replaced by the Raft commit handler
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
        .ok_or_else(|| RaftMetricsError::NotFound(format!("Metric {} not found", name)))?;

    Ok(Json(WorkerMetricResponse {
        name,
        value,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

async fn get_metric_aggregate(
    State(state): State<WorkerState>,
    Path(name): Path<String>,
) -> Result<Json<MetricAggregateResponse>> {
    info!("Worker {} calculating aggregate for metric: {}", state.worker_id, name);
    
    let stats = state.metrics.get_metric_aggregate(&name).await?
        .ok_or_else(|| RaftMetricsError::NotFound(format!("Metric {} not found", name)))?;

    Ok(Json(MetricAggregateResponse {
        name,
        count: stats.count,
        sum: stats.sum,
        average: stats.average,
        min: stats.min,
        max: stats.max,
    }))
}

pub async fn start_worker_node(worker_id: usize) -> Result<()> {
    let storage = Arc::new(MemStorage::new());
    let metrics = Arc::new(MetricsRegistry::new()?);

    // Set up Raft channels
    let (proposal_tx, proposal_rx) = mpsc::channel(100);
    let (msg_tx, msg_rx) = mpsc::channel(100);

    // Parse Raft cluster configuration
    let raft_cluster = env::var("RAFT_CLUSTER")
        .unwrap_or_else(|_| "worker-1:9081,worker-2:9082".to_string());
    
    let mut peers = HashMap::new();
    for (id, addr) in raft_cluster.split(',').enumerate() {
        peers.insert((id + 1) as u64, addr.to_string());
    }

    // Initialize Raft node
    let raft_node = RaftNode::new(
        worker_id as u64,
        peers,
        msg_tx,
        metrics.clone(),
    ).expect("Failed to create Raft node");

    // Start Raft node in background
    tokio::spawn(run_raft_node(raft_node, proposal_rx, msg_rx));

    // Create worker state
    let state = WorkerState {
        storage: storage.clone(),
        metrics: metrics.clone(),
        worker_id,
        proposal_tx,
    };

    // Start HTTP server
    let app = worker_router(state);
    let port = env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("Starting worker node {} on {}", worker_id, addr);

    let listener = TcpListener::bind(addr).await.map_err(|e| 
        RaftMetricsError::Internal(format!("Failed to bind to address: {}", e)))?;

    axum::serve(listener, app).await.map_err(|e|
        RaftMetricsError::Internal(format!("Server error: {}", e)))?;

    Ok(())
}
