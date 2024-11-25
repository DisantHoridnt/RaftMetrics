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
    metrics::{MetricsRegistry, MetricOperation},
    raft::{
        node::{RaftNode, run_raft_node},
        storage::MemStorage,
    },
};

#[derive(Clone)]
pub struct ControlState {
    pub storage: Arc<MemStorage>,
    pub metrics: Arc<MetricsRegistry>,
    pub proposal_tx: mpsc::Sender<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricRequest {
    pub metric_name: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricResponse {
    pub name: String,
    pub value: f64,
    pub timestamp: i64,
}

async fn control_router() -> Router {
    Router::new()
        .route("/metrics", post(record_metric))
        .route("/metrics/:name", get(get_metric))
        .route("/metrics/:name/aggregate", get(get_metric_aggregate))
}

async fn record_metric(
    State(state): State<ControlState>,
    Json(request): Json<MetricRequest>,
) -> Result<Json<MetricResponse>> {
    info!("Recording metric: {} = {}", request.metric_name, request.value);

    // Create Raft operation
    let operation = MetricOperation::Record {
        name: request.metric_name.clone(),
        value: request.value,
    };

    // Serialize operation
    let data = MetricsRegistry::serialize_operation(&operation)?;

    // Send proposal through Raft
    state.proposal_tx.send(data).await.map_err(|e| {
        RaftMetricsError::Internal(format!("Failed to send proposal: {}", e))
    })?;

    Ok(Json(MetricResponse {
        name: request.metric_name,
        value: request.value,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

async fn get_metric(
    State(state): State<ControlState>,
    Path(name): Path<String>,
) -> Result<Json<MetricResponse>> {
    info!("Getting metric: {}", name);
    
    let value = state.metrics.get_metric(&name).await?
        .ok_or_else(|| RaftMetricsError::NotFound(format!("Metric {} not found", name)))?;

    Ok(Json(MetricResponse {
        name,
        value,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

async fn get_metric_aggregate(
    State(state): State<ControlState>,
    Path(name): Path<String>,
) -> Result<Json<MetricAggregateResponse>> {
    info!("Getting metric aggregate: {}", name);
    
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

pub async fn start_control_node() -> Result<()> {
    let storage = Arc::new(MemStorage::new());
    let metrics = Arc::new(MetricsRegistry::new()?);

    // Set up Raft channels
    let (proposal_tx, proposal_rx) = mpsc::channel(100);
    let (msg_tx, msg_rx) = mpsc::channel(100);

    // Parse Raft cluster configuration
    let raft_cluster = env::var("RAFT_CLUSTER")
        .unwrap_or_else(|_| "control:9080,worker-1:9081,worker-2:9082".to_string());
    
    let mut peers = HashMap::new();
    for (id, addr) in raft_cluster.split(',').enumerate() {
        peers.insert((id + 1) as u64, addr.to_string());
    }

    // Initialize Raft node
    let raft_node = RaftNode::new(
        1, // Control node is always ID 1
        peers,
        msg_tx,
        metrics.clone(),
    ).expect("Failed to create Raft node");

    // Start Raft node in background
    tokio::spawn(run_raft_node(raft_node, proposal_rx, msg_rx));

    // Create control state
    let state = ControlState {
        storage: storage.clone(),
        metrics: metrics.clone(),
        proposal_tx,
    };

    // Start HTTP server
    let app = control_router().await.with_state(state);
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("Starting control node on {}", addr);

    let listener = TcpListener::bind(addr).await.map_err(|e| 
        RaftMetricsError::Internal(format!("Failed to bind to address: {}", e)))?;

    axum::serve(listener, app).await.map_err(|e|
        RaftMetricsError::Internal(format!("Server error: {}", e)))?;

    Ok(())
}
